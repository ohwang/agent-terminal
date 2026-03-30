#!/usr/bin/env node

import { execSync, spawn } from "node:child_process";
import { chmodSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { platform, arch } from "node:os";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function detectMusl() {
  if (platform() !== "linux") return false;

  // Check /etc/os-release for Alpine or other musl-based distros
  try {
    const osRelease = execSync("cat /etc/os-release 2>/dev/null", {
      encoding: "utf-8",
    });
    if (/alpine|musl/i.test(osRelease)) return true;
  } catch {}

  // Check if the system linker is musl-based
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

  if (os === "win32") {
    console.error(
      "Error: agent-terminal requires tmux, which is not available on Windows."
    );
    console.error("Please use WSL (Windows Subsystem for Linux) instead.");
    process.exit(1);
  }

  if (os !== "darwin" && os !== "linux") {
    console.error(`Error: Unsupported platform: ${os}`);
    console.error("agent-terminal supports macOS (darwin) and Linux.");
    process.exit(1);
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
      console.error(`Error: Unsupported architecture: ${cpuArch}`);
      console.error("agent-terminal supports x64 and arm64.");
      process.exit(1);
  }

  const musl = os === "linux" && detectMusl();
  const platformSegment = musl ? `${os}-musl` : os;

  return `agent-terminal-${platformSegment}-${archName}`;
}

function main() {
  const binaryName = getPlatformTarget();
  const binaryPath = join(__dirname, binaryName);

  if (!existsSync(binaryPath)) {
    console.error(`Error: Binary not found at ${binaryPath}`);
    console.error("");
    console.error("The native binary for your platform was not installed.");
    console.error("Try reinstalling:");
    console.error("");
    console.error("  npm install -g agent-terminal");
    console.error("");
    console.error(
      "If that doesn't work, you can build from source:"
    );
    console.error("");
    console.error(
      "  git clone https://github.com/ohwang/agent-terminal.git"
    );
    console.error("  cd agent-terminal");
    console.error("  cargo build --release");
    console.error("");
    process.exit(1);
  }

  // Ensure the binary is executable
  try {
    chmodSync(binaryPath, 0o755);
  } catch {
    // May fail if we don't own the file, but it might already be executable
  }

  const args = process.argv.slice(2);
  const child = spawn(binaryPath, args, {
    stdio: "inherit",
  });

  child.on("error", (err) => {
    if (err.code === "EACCES") {
      console.error(`Error: Permission denied executing ${binaryPath}`);
      console.error("Try: chmod +x " + binaryPath);
    } else {
      console.error(`Error: Failed to start agent-terminal: ${err.message}`);
    }
    process.exit(1);
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      // Re-raise the signal so the parent process sees the correct exit reason
      process.kill(process.pid, signal);
    } else {
      process.exit(code ?? 1);
    }
  });
}

main();
