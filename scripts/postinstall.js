#!/usr/bin/env node

import { execSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  readFileSync,
  unlinkSync,
  symlinkSync,
  lstatSync,
  createWriteStream,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { platform, arch } from "node:os";
import https from "node:https";
import http from "node:http";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = resolve(__dirname, "..");
const BIN_DIR = join(ROOT, "bin");

// ---------------------------------------------------------------------------
// Platform detection
// ---------------------------------------------------------------------------

function detectMusl() {
  if (platform() !== "linux") return false;

  try {
    const osRelease = execSync("cat /etc/os-release 2>/dev/null", {
      encoding: "utf-8",
    });
    if (/alpine|musl/i.test(osRelease)) return true;
  } catch {}

  try {
    const lddOutput = execSync("ldd --version 2>&1 || true", {
      encoding: "utf-8",
    });
    if (/musl/i.test(lddOutput)) return true;
  } catch {}

  return false;
}

function getPlatformTarget() {
  const os = platform();
  const cpuArch = arch();

  if (os !== "darwin" && os !== "linux") {
    return null;
  }

  let archName;
  switch (cpuArch) {
    case "x64":
      archName = "x64";
      break;
    case "arm64":
      archName = "arm64";
      break;
    default:
      return null;
  }

  const musl = os === "linux" && detectMusl();
  const platformSegment = musl ? `${os}-musl` : os;

  return `agent-terminal-${platformSegment}-${archName}`;
}

// ---------------------------------------------------------------------------
// Download helpers (Node.js built-in https, follow redirects)
// ---------------------------------------------------------------------------

function download(url, dest) {
  return new Promise((resolvePromise, reject) => {
    const file = createWriteStream(dest);
    const client = url.startsWith("https") ? https : http;

    const request = (currentUrl) => {
      client
        .get(currentUrl, (res) => {
          // Follow redirects (GitHub releases use 302)
          if (
            (res.statusCode === 301 || res.statusCode === 302) &&
            res.headers.location
          ) {
            res.resume(); // consume the response body
            const location = res.headers.location;
            const redirectClient = location.startsWith("https") ? https : http;
            redirectClient
              .get(location, (redirectRes) => {
                if (
                  (redirectRes.statusCode === 301 ||
                    redirectRes.statusCode === 302) &&
                  redirectRes.headers.location
                ) {
                  // Follow one more redirect (GitHub often does two)
                  redirectRes.resume();
                  const finalClient = redirectRes.headers.location.startsWith(
                    "https"
                  )
                    ? https
                    : http;
                  finalClient
                    .get(redirectRes.headers.location, (finalRes) => {
                      if (finalRes.statusCode !== 200) {
                        file.close();
                        reject(
                          new Error(
                            `Download failed with status ${finalRes.statusCode}`
                          )
                        );
                        return;
                      }
                      finalRes.pipe(file);
                      file.on("finish", () => {
                        file.close(resolvePromise);
                      });
                    })
                    .on("error", (err) => {
                      file.close();
                      reject(err);
                    });
                  return;
                }
                if (redirectRes.statusCode !== 200) {
                  file.close();
                  reject(
                    new Error(
                      `Download failed with status ${redirectRes.statusCode}`
                    )
                  );
                  return;
                }
                redirectRes.pipe(file);
                file.on("finish", () => {
                  file.close(resolvePromise);
                });
              })
              .on("error", (err) => {
                file.close();
                reject(err);
              });
            return;
          }

          if (res.statusCode !== 200) {
            file.close();
            reject(
              new Error(`Download failed with status ${res.statusCode}`)
            );
            return;
          }

          res.pipe(file);
          file.on("finish", () => {
            file.close(resolvePromise);
          });
        })
        .on("error", (err) => {
          file.close();
          reject(err);
        });
    };

    request(url);
  });
}

// ---------------------------------------------------------------------------
// A. Download binary if missing
// ---------------------------------------------------------------------------

async function ensureBinary() {
  const target = getPlatformTarget();
  if (!target) {
    console.log(
      "agent-terminal: Unsupported platform/architecture, skipping binary download."
    );
    return null;
  }

  const binaryPath = join(BIN_DIR, target);

  if (existsSync(binaryPath)) {
    // Already present (e.g. shipped in the package or previously downloaded)
    chmodSync(binaryPath, 0o755);
    return binaryPath;
  }

  // Read version from package.json
  const pkg = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf-8"));
  const version = pkg.version;

  const url = `https://github.com/anthropics/agent-terminal/releases/download/v${version}/${target}`;

  console.log(`agent-terminal: Downloading native binary for ${target}...`);
  console.log(`  ${url}`);

  try {
    await download(url, binaryPath);
    chmodSync(binaryPath, 0o755);
    console.log("agent-terminal: Binary installed successfully.");
    return binaryPath;
  } catch (err) {
    console.error(`agent-terminal: Failed to download binary: ${err.message}`);
    console.error("");
    console.error("You can build from source instead:");
    console.error("");
    console.error(
      "  git clone https://github.com/anthropics/agent-terminal.git"
    );
    console.error("  cd agent-terminal");
    console.error("  cargo build --release");
    console.error(
      `  cp target/release/agent-terminal ${binaryPath}`
    );
    console.error("");

    // Clean up partial download
    try {
      unlinkSync(binaryPath);
    } catch {}

    return null;
  }
}

// ---------------------------------------------------------------------------
// B. Global install optimization -- replace npm shim with direct symlink
// ---------------------------------------------------------------------------

function optimizeGlobalInstall(binaryPath) {
  if (!binaryPath) return;

  // npm sets npm_config_global=true for global installs
  const isGlobal =
    process.env.npm_config_global === "true" ||
    // yarn global
    (process.env.npm_lifecycle_event === "postinstall" &&
      process.env.npm_config_prefix &&
      !process.env.npm_config_prefix.includes("node_modules"));

  if (!isGlobal) return;

  try {
    // Find where npm put the shim
    const npmBin = execSync("npm bin -g", { encoding: "utf-8" }).trim();
    const shimPath = join(npmBin, "agent-terminal");

    if (!existsSync(shimPath)) return;

    const shimStat = lstatSync(shimPath);

    // Only replace if it's a symlink (npm creates symlinks) or a regular file
    // (npm on some systems creates wrapper scripts)
    if (shimStat.isSymbolicLink() || shimStat.isFile()) {
      try {
        unlinkSync(shimPath);
        symlinkSync(binaryPath, shimPath);
        console.log(
          "agent-terminal: Optimized global install (direct symlink to native binary)."
        );
      } catch (err) {
        // If we can't replace the shim, the JS wrapper still works fine
        // This is a performance optimization, not a requirement
      }
    }
  } catch {
    // Non-critical -- the JS shim wrapper works fine as a fallback
  }
}

// ---------------------------------------------------------------------------
// C. Post-install messages -- check for tmux
// ---------------------------------------------------------------------------

function checkTmux() {
  console.log("");

  try {
    const tmuxVersion = execSync("tmux -V 2>/dev/null", {
      encoding: "utf-8",
    }).trim();
    console.log(`agent-terminal: Found ${tmuxVersion}`);
  } catch {
    console.warn("agent-terminal: tmux not found!");
    console.warn("");
    console.warn("  agent-terminal requires tmux to operate.");
    console.warn("");

    if (platform() === "darwin") {
      console.warn("  Install with Homebrew:");
      console.warn("    brew install tmux");
    } else {
      console.warn("  Install with your package manager:");
      console.warn("    apt-get install tmux    # Debian/Ubuntu");
      console.warn("    yum install tmux        # RHEL/CentOS");
      console.warn("    apk add tmux            # Alpine");
    }
    console.warn("");
  }

  console.log("");
  console.log("  Run `agent-terminal doctor` to verify your setup.");
  console.log("");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const binaryPath = await ensureBinary();
  optimizeGlobalInstall(binaryPath);
  checkTmux();
}

main().catch((err) => {
  console.error("agent-terminal postinstall error:", err.message);
  // Don't fail the install -- the user can still build from source
  process.exit(0);
});
