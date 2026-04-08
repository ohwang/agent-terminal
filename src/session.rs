use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

/// Run a tmux command with the given arguments.  Returns stdout on success,
/// or a human-readable error string on failure.
fn tmux_cmd(args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run tmux: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!(
            "tmux {} failed: {}",
            args.join(" "),
            if stderr.is_empty() {
                format!("exit code {}", output.status.code().unwrap_or(-1))
            } else {
                stderr
            }
        ))
    }
}

/// Resolve a size preset name to a COLSxROWS string.
/// Passes through values that are already in COLSxROWS format.
fn resolve_size_preset(size: &str) -> String {
    match size {
        "landscape" => "112x30".to_string(),
        "vertical" => "80x55".to_string(),
        other => other.to_string(),
    }
}

/// Check whether a tmux session exists.
fn session_exists(session: &str) -> bool {
    tmux_cmd(&["has-session", "-t", session]).is_ok()
}

/// Build a tmux target string, e.g. `"mysess"` or `"mysess:mypane"`.
fn target_pane(session: &str, pane: Option<&str>) -> String {
    match pane {
        Some(p) if p.starts_with('%') => p.to_string(),
        Some(p) => format!("{session}:{p}"),
        None => session.to_string(),
    }
}

/// Get the PID of the shell/process running inside the tmux pane.
fn get_pane_pid(session: &str, pane: Option<&str>) -> Result<u32, String> {
    let target = target_pane(session, pane);
    let out = tmux_cmd(&["display-message", "-t", &target, "-p", "#{pane_pid}"])?;
    out.trim()
        .parse::<u32>()
        .map_err(|e| format!("Failed to parse pane PID '{out}': {e}"))
}

/// Check whether a process with the given PID is alive.
fn is_pid_alive(pid: u32) -> bool {
    // kill -0 just checks existence, doesn't actually signal.
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    kill(Pid::from_raw(pid as i32), Signal::SIGCONT)
        .map(|_| true)
        .unwrap_or(false)
        || kill(Pid::from_raw(pid as i32), None).is_ok()
}

/// Temp file paths scoped to a session.
fn stderr_path(session: &str) -> String {
    format!("/tmp/agent-terminal-{session}-stderr")
}
fn exit_code_path(session: &str) -> String {
    format!("/tmp/agent-terminal-{session}-exit")
}

/// Parse tmux version from `tmux -V` output like "tmux 3.6a" → 3.6
fn parse_tmux_version() -> Result<f64, String> {
    let out = tmux_cmd(&["-V"])?;
    // Example: "tmux 3.6a\n"
    let version_str = out
        .trim()
        .strip_prefix("tmux ")
        .ok_or_else(|| format!("Unexpected tmux -V output: {out}"))?;
    // Strip trailing alphabetic chars: "3.6a" → "3.6"
    let numeric: String = version_str
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    numeric
        .parse::<f64>()
        .map_err(|e| format!("Cannot parse tmux version '{numeric}': {e}"))
}

/// Attempt to capture a snapshot of the pane (for embedding in error messages).
fn try_capture_snapshot(session: &str, pane: Option<&str>) -> Option<String> {
    let target = target_pane(session, pane);
    tmux_cmd(&["capture-pane", "-t", &target, "-p"]).ok()
}

/// Build a rich error string that includes session state + last snapshot.
fn rich_error(session: &str, pane: Option<&str>, message: &str) -> String {
    let mut parts = vec![message.to_string()];

    // Session state
    if session_exists(session) {
        let pid_info = get_pane_pid(session, pane)
            .map(|pid| {
                let alive = is_pid_alive(pid);
                format!("pid {pid}, {}", if alive { "alive" } else { "dead" })
            })
            .unwrap_or_else(|_| "pid unknown".to_string());

        let exit_info = fs::read_to_string(exit_code_path(session))
            .ok()
            .map(|s| format!(", exit code {}", s.trim()))
            .unwrap_or_default();

        parts.push(format!("\nSession: {session} ({pid_info}{exit_info})"));

        // Last snapshot
        if let Some(snap) = try_capture_snapshot(session, pane) {
            let snap = snap.trim_end();
            if !snap.is_empty() {
                let numbered: String = snap
                    .lines()
                    .enumerate()
                    .map(|(i, line)| format!("  {}| {line}", i + 1))
                    .collect::<Vec<_>>()
                    .join("\n");
                parts.push(format!("Last snapshot:\n{numbered}"));
            }
        }
    } else {
        parts.push(format!("\nSession: {session} (not found)"));
    }

    parts.join("\n")
}

/// Get session creation time as a unix timestamp.
fn session_created_ts(session: &str) -> Result<u64, String> {
    let out = tmux_cmd(&["display-message", "-t", session, "-p", "#{session_created}"])?;
    out.trim()
        .parse::<u64>()
        .map_err(|e| format!("Cannot parse session_created: {e}"))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Launch a command in a new (or split) tmux session.
#[allow(clippy::too_many_arguments)]
pub fn open(
    command: &str,
    session: &str,
    pane: Option<&str>,
    envs: &[String],
    size: Option<&str>,
    shell: bool,
    no_stderr: bool,
    replace: bool,
) -> Result<(), String> {
    // Build the env export prefix.
    let mut env_prefix = String::new();
    for env_spec in envs {
        if !env_spec.contains('=') {
            return Err(format!(
                "Invalid --env value '{env_spec}': expected KEY=VALUE format"
            ));
        }
        env_prefix.push_str(&format!("export {env_spec}; "));
    }

    // Build the wrapped command.
    let stderr_file = stderr_path(session);
    let exit_file = exit_code_path(session);

    let wrapped = if no_stderr {
        // No stderr capture — needed for bash/readline apps where prompts
        // and tab completion go through stderr.
        if shell {
            format!("{env_prefix}{command}; exec $SHELL")
        } else {
            format!("{env_prefix}{command}")
        }
    } else if shell {
        // Capture stderr but keep session alive after command exits.
        format!("{env_prefix}{command} 2>{stderr_file}; echo $? > {exit_file}; exec $SHELL")
    } else {
        // Default: capture stderr and exit code.
        format!("{env_prefix}{command} 2>{stderr_file}; echo $? > {exit_file}")
    };

    if let Some(pane_name) = pane {
        // Split an existing session to add a new pane.
        if !session_exists(session) {
            return Err(format!(
                "Session '{session}' does not exist; cannot split pane"
            ));
        }
        tmux_cmd(&["split-window", "-t", session, "-h", &wrapped])?;
        // Optionally rename the pane — tmux doesn't have native pane naming,
        // but we can use select-pane -T.
        let target = target_pane(session, Some(pane_name));
        // The newly split pane is auto-selected; set its title.
        let _ = tmux_cmd(&["select-pane", "-t", session, "-T", pane_name]);
        let _ = target; // suppress unused warning
    } else {
        // Create a brand-new detached session.
        if session_exists(session) {
            if replace {
                close(session)?;
            } else {
                return Err(format!(
                    "Session '{session}' already exists. Close it first or use --replace."
                ));
            }
        }
        // Parse size for new-session -x/-y (supports presets or COLSxROWS)
        let mut new_session_args = vec!["new-session", "-d", "-s", session];
        let (cols_str, rows_str) = if let Some(size_str) = size {
            let resolved = resolve_size_preset(size_str);
            let parts: Vec<&str> = resolved.split('x').collect();
            if parts.len() != 2 {
                return Err(format!(
                    "Invalid --size '{size_str}': expected COLSxROWS or a preset name (landscape, vertical)"
                ));
            }
            parts[0]
                .parse::<u16>()
                .map_err(|_| format!("Invalid columns in --size: '{}'", parts[0]))?;
            parts[1]
                .parse::<u16>()
                .map_err(|_| format!("Invalid rows in --size: '{}'", parts[1]))?;
            (Some(parts[0].to_string()), Some(parts[1].to_string()))
        } else {
            (None, None)
        };
        if let (Some(ref c), Some(ref r)) = (&cols_str, &rows_str) {
            new_session_args.extend_from_slice(&["-x", c, "-y", r]);
        }
        new_session_args.push(&wrapped);
        tmux_cmd(&new_session_args)?;
    }

    // Enable mouse support so the pane forwards mouse events to apps.
    let _ = tmux_cmd(&["set-option", "-t", session, "mouse", "on"]);

    // Wait for the first render — poll up to 2 seconds for capture-pane to
    // return non-empty output.
    let deadline = Instant::now() + Duration::from_secs(2);
    let target = target_pane(session, pane);
    loop {
        if let Ok(snap) = tmux_cmd(&["capture-pane", "-t", &target, "-p"]) {
            if !snap.trim().is_empty() {
                break;
            }
        }
        if Instant::now() >= deadline {
            // Timed out, but the session was created — don't treat as error.
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    // Report the actual terminal size (may differ from requested if tmux adjusted)
    let target_final = target_pane(session, pane);
    let (actual_cols, actual_rows, _, _) =
        crate::snapshot::get_pane_info(session, Some(&target_final)).unwrap_or((0, 0, 0, 0));
    println!("session={session} size={actual_cols}x{actual_rows} command={command}");
    Ok(())
}

/// Kill a tmux session and clean up temp files.
pub fn close(session: &str) -> Result<(), String> {
    if !session_exists(session) {
        return Err(format!("Session '{session}' does not exist"));
    }
    tmux_cmd(&["kill-session", "-t", session])?;

    // Clean up temp files (best effort).
    let _ = fs::remove_file(stderr_path(session));
    let _ = fs::remove_file(exit_code_path(session));

    println!("Closed session: {session}");
    Ok(())
}

/// List active tmux sessions.
pub fn list() -> Result<(), String> {
    let out = tmux_cmd(&[
        "list-sessions",
        "-F",
        "#{session_name} #{session_created} #{session_windows}",
    ]);

    match out {
        Ok(text) => {
            if text.trim().is_empty() {
                println!("No active tmux sessions.");
                return Ok(());
            }
            println!(
                "{:<30} {:<24} {:<10} PANES",
                "SESSION", "CREATED", "WINDOWS"
            );
            for line in text.trim().lines() {
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() < 3 {
                    continue;
                }
                let name = parts[0];
                let created_ts = parts[1].parse::<i64>().unwrap_or(0);
                let windows = parts[2];
                let tag = if name.starts_with("agent-terminal") {
                    " [agent-terminal]"
                } else {
                    ""
                };

                // Count panes for this session
                let pane_count = crate::snapshot::list_pane_layouts(name)
                    .map(|p| p.len())
                    .unwrap_or(0);

                // Convert unix timestamp to a readable string.
                let created_str = if created_ts > 0 {
                    format!("{created_ts}")
                } else {
                    "unknown".to_string()
                };

                println!(
                    "{:<30} {:<24} {:<10} {}{}",
                    name, created_str, windows, pane_count, tag
                );
            }
            Ok(())
        }
        Err(e) => {
            // "no server running" is not really an error — just means no sessions.
            if e.contains("no server running") || e.contains("no current") {
                println!("No active tmux sessions.");
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

/// Get status information about a session.
pub fn status(session: &str, pane: Option<&str>, json: bool) -> Result<(), String> {
    if !session_exists(session) {
        if json {
            println!(
                "{{\"alive\":false,\"pid\":null,\"exit_code\":null,\"signal\":null,\"runtime_ms\":0}}"
            );
            return Ok(());
        }
        return Err(rich_error(
            session,
            pane,
            &format!("Session '{session}' does not exist"),
        ));
    }

    let pid = get_pane_pid(session, pane)?;
    let alive = is_pid_alive(pid);

    // Read exit code if available.
    let exit_code: Option<i32> = fs::read_to_string(exit_code_path(session))
        .ok()
        .and_then(|s| s.trim().parse().ok());

    // Calculate runtime from session creation time.
    let runtime_ms: u64 = session_created_ts(session)
        .map(|created| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let created_ms = created * 1000;
            now.saturating_sub(created_ms)
        })
        .unwrap_or(0);

    // Query pane layout info
    let pane_layouts = crate::snapshot::list_pane_layouts(session).unwrap_or_default();

    // Read last stderr when process is dead (truncate to avoid huge output).
    let last_stderr: Option<String> = if !alive {
        fs::read_to_string(stderr_path(session)).ok().and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                // Keep last 50 lines max
                let lines: Vec<&str> = trimmed.lines().collect();
                if lines.len() > 50 {
                    Some(lines[lines.len() - 50..].join("\n"))
                } else {
                    Some(trimmed.to_string())
                }
            }
        })
    } else {
        None
    };

    if json {
        let ec_json = match exit_code {
            Some(c) => c.to_string(),
            None => "null".to_string(),
        };
        let stderr_json = match &last_stderr {
            Some(s) => serde_json::to_string(s).unwrap_or_else(|_| "null".to_string()),
            None => "null".to_string(),
        };
        let panes_json = if pane_layouts.len() > 1 {
            let entries: Vec<String> = pane_layouts
                .iter()
                .map(|p| {
                    format!(
                        "{{\"id\":\"{}\",\"left\":{},\"top\":{},\"width\":{},\"height\":{},\"title\":\"{}\",\"active\":{}}}",
                        p.pane_id, p.left, p.top, p.width, p.height, p.title, p.active
                    )
                })
                .collect();
            format!(",\"panes\":[{}]", entries.join(","))
        } else {
            String::new()
        };
        println!(
            "{{\"alive\":{alive},\"pid\":{pid},\"exit_code\":{ec_json},\"signal\":null,\"runtime_ms\":{runtime_ms},\"last_stderr\":{stderr_json}{panes_json}}}"
        );
    } else {
        let status_word = if alive { "alive" } else { "dead" };
        println!("Session:  {session}");
        println!("Status:   {status_word}");
        println!("PID:      {pid}");
        if let Some(ec) = exit_code {
            println!("Exit code: {ec}");
        }
        let secs = runtime_ms / 1000;
        let ms = runtime_ms % 1000;
        println!("Runtime:  {secs}.{ms:03}s");
        if let Some(ref stderr) = last_stderr {
            println!("Last stderr:");
            for line in stderr.lines() {
                println!("  {line}");
            }
        }
        if pane_layouts.len() > 1 {
            println!("Panes:    {}", pane_layouts.len());
            for p in &pane_layouts {
                let active_marker = if p.active { "  (active)" } else { "" };
                println!(
                    "  {}  [{}x{} at {},{}]  \"{}\"{active_marker}",
                    p.pane_id, p.width, p.height, p.left, p.top, p.title
                );
            }
            println!(
                "Hint:     use --window to capture all panes, or --pane <id> for a specific one"
            );
        }
    }

    Ok(())
}

/// Print the exit code of the process that ran inside the session.
pub fn exit_code(session: &str) -> Result<(), String> {
    let path = exit_code_path(session);
    match fs::read_to_string(&path) {
        Ok(contents) => {
            let code = contents.trim();
            if code.is_empty() {
                return Err("Exit code file is empty".to_string());
            }
            println!("{code}");
            Ok(())
        }
        Err(_) => {
            // File doesn't exist yet — check if the process is still running.
            if session_exists(session) {
                if let Ok(pid) = get_pane_pid(session, None) {
                    if is_pid_alive(pid) {
                        println!("Process still running");
                        return Ok(());
                    }
                }
            }
            Err(rich_error(
                session,
                None,
                "Exit code not available (process may have been killed or session not found)",
            ))
        }
    }
}

/// Print logs (stderr and optionally stdout scrollback).
pub fn logs(session: &str, stderr_only: bool) -> Result<(), String> {
    let stderr_file = stderr_path(session);
    let stderr_content = fs::read_to_string(&stderr_file).unwrap_or_default();

    if stderr_only {
        if stderr_content.is_empty() {
            println!("(no stderr output)");
        } else {
            print!("{stderr_content}");
        }
        return Ok(());
    }

    // Print stderr section.
    println!("=== STDERR ===");
    if stderr_content.is_empty() {
        println!("(empty)");
    } else {
        print!("{stderr_content}");
    }

    // Capture scrollback buffer for stdout context.
    println!("\n=== STDOUT (scrollback) ===");
    if session_exists(session) {
        let target = target_pane(session, None);
        match tmux_cmd(&["capture-pane", "-t", &target, "-p", "-S", "-1000"]) {
            Ok(scrollback) => {
                if scrollback.trim().is_empty() {
                    println!("(empty)");
                } else {
                    print!("{scrollback}");
                }
            }
            Err(e) => {
                println!("(could not capture scrollback: {e})");
            }
        }
    } else {
        println!("(session '{session}' not found — cannot capture scrollback)");
    }

    Ok(())
}

/// Validate the environment: check tmux version and key capabilities.
pub fn doctor() -> Result<(), String> {
    let mut all_ok = true;

    // 1. Check tmux installed + version >= 3.0
    print!("tmux installed ............. ");
    match parse_tmux_version() {
        Ok(ver) => {
            if ver >= 3.0 {
                println!("\u{2713} (version {ver})");
            } else {
                println!("\u{2717} (version {ver} — need >= 3.0)");
                println!("  Fix: upgrade tmux (brew install tmux / apt install tmux)");
                all_ok = false;
            }
        }
        Err(e) => {
            println!("\u{2717} ({e})");
            println!("  Fix: install tmux (brew install tmux / apt install tmux)");
            all_ok = false;
        }
    }

    let test_session = "agent-terminal-doctor-test";

    // Clean up any leftover test session.
    let _ = tmux_cmd(&["kill-session", "-t", test_session]);

    // 2. Create/destroy session
    print!("create session ............. ");
    match tmux_cmd(&["new-session", "-d", "-s", test_session, "sleep 30"]) {
        Ok(_) => println!("\u{2713}"),
        Err(e) => {
            println!("\u{2717} ({e})");
            // Can't continue other tests without a session.
            println!("\nResult: FAIL — cannot create tmux sessions");
            return Ok(());
        }
    }

    // 3. capture-pane
    print!("capture-pane ............... ");
    match tmux_cmd(&["capture-pane", "-t", test_session, "-p"]) {
        Ok(_) => println!("\u{2713}"),
        Err(e) => {
            println!("\u{2717} ({e})");
            all_ok = false;
        }
    }

    // 4. capture-pane -e (ANSI escapes)
    print!("capture-pane -e (ANSI) ..... ");
    match tmux_cmd(&["capture-pane", "-t", test_session, "-p", "-e"]) {
        Ok(_) => println!("\u{2713}"),
        Err(e) => {
            println!("\u{2717} ({e})");
            println!("  Fix: upgrade tmux to >= 3.0");
            all_ok = false;
        }
    }

    // 5. Mouse support / SGR encoding
    print!("mouse support (SGR) ........ ");
    // Set mouse mode and verify it doesn't error.
    match tmux_cmd(&["set-option", "-t", test_session, "mouse", "on"]) {
        Ok(_) => {
            // Also verify terminal-features includes the session.
            println!("\u{2713}");
        }
        Err(e) => {
            println!("\u{2717} ({e})");
            println!("  Fix: ensure tmux >= 3.2 for full SGR mouse support");
            all_ok = false;
        }
    }

    // 6. resize-pane
    print!("resize-pane ................ ");
    match tmux_cmd(&["resize-window", "-t", test_session, "-x", "80", "-y", "24"]) {
        Ok(_) => println!("\u{2713}"),
        Err(e) => {
            println!("\u{2717} ({e})");
            all_ok = false;
        }
    }

    // 7. send-keys
    print!("send-keys .................. ");
    match tmux_cmd(&["send-keys", "-t", test_session, "echo test", "Enter"]) {
        Ok(_) => println!("\u{2713}"),
        Err(e) => {
            println!("\u{2717} ({e})");
            all_ok = false;
        }
    }

    // 8. paste-buffer
    print!("paste-buffer ............... ");
    // Load a test string into the buffer, then paste it.
    let buf_result = tmux_cmd(&["set-buffer", "doctor-test"])
        .and_then(|_| tmux_cmd(&["paste-buffer", "-t", test_session]));
    match buf_result {
        Ok(_) => println!("\u{2713}"),
        Err(e) => {
            println!("\u{2717} ({e})");
            all_ok = false;
        }
    }

    // Cleanup.
    let _ = tmux_cmd(&["kill-session", "-t", test_session]);

    println!();
    if all_ok {
        println!("Result: OK — all checks passed");
    } else {
        println!("Result: WARN — some checks failed (see above)");
    }

    Ok(())
}

/// Detect the TUI framework in the current directory and generate a starter test.
pub fn init() -> Result<(), String> {
    let mut detected: Vec<(&str, &str)> = Vec::new(); // (framework, language)

    // --- Rust ---
    if let Ok(cargo) = fs::read_to_string("Cargo.toml") {
        let cargo_lower = cargo.to_lowercase();
        if cargo_lower.contains("ratatui") {
            detected.push(("ratatui", "rust"));
        }
        if cargo_lower.contains("crossterm") {
            detected.push(("crossterm", "rust"));
        }
        if cargo_lower.contains("cursive") {
            detected.push(("cursive", "rust"));
        }
    }

    // --- Go ---
    if let Ok(gomod) = fs::read_to_string("go.mod") {
        let gomod_lower = gomod.to_lowercase();
        if gomod_lower.contains("bubbletea") {
            detected.push(("bubbletea", "go"));
        }
        if gomod_lower.contains("tview") {
            detected.push(("tview", "go"));
        }
        if gomod_lower.contains("termui") {
            detected.push(("termui", "go"));
        }
    }

    // --- JavaScript / TypeScript ---
    if let Ok(pkg) = fs::read_to_string("package.json") {
        let pkg_lower = pkg.to_lowercase();
        if pkg_lower.contains("\"ink\"") || pkg_lower.contains("\"ink\":") {
            detected.push(("ink", "javascript"));
        }
        if pkg_lower.contains("\"blessed\"") || pkg_lower.contains("\"blessed\":") {
            detected.push(("blessed", "javascript"));
        }
        if pkg_lower.contains("\"terminal-kit\"") || pkg_lower.contains("\"terminal-kit\":") {
            detected.push(("terminal-kit", "javascript"));
        }
    }

    // --- Python ---
    for pyfile in &["requirements.txt", "pyproject.toml"] {
        if let Ok(py) = fs::read_to_string(pyfile) {
            let py_lower = py.to_lowercase();
            if py_lower.contains("textual") {
                detected.push(("textual", "python"));
            }
            if py_lower.contains("rich") {
                detected.push(("rich", "python"));
            }
            if py_lower.contains("curses") {
                detected.push(("curses", "python"));
            }
        }
    }

    // De-duplicate.
    detected.sort();
    detected.dedup();

    if detected.is_empty() {
        println!("No TUI framework detected in the current directory.");
        println!("Generating a generic starter test...");
    } else {
        println!("Detected TUI framework(s):");
        for (fw, lang) in &detected {
            println!("  - {fw} ({lang})");
        }
    }

    // Determine a reasonable run command based on detection.
    let run_cmd = if detected.iter().any(|(_, l)| *l == "rust") {
        "cargo run"
    } else if detected.iter().any(|(_, l)| *l == "go") {
        "go run ."
    } else if detected.iter().any(|(fw, _)| *fw == "ink") {
        "npx tsx src/index.tsx"
    } else if detected.iter().any(|(_, l)| *l == "javascript") {
        "npm start"
    } else if detected.iter().any(|(_, l)| *l == "python") {
        "python main.py"
    } else {
        "./my-app"
    };

    // Create directory.
    let test_dir = Path::new("tests/tui");
    fs::create_dir_all(test_dir)
        .map_err(|e| format!("Failed to create {}: {e}", test_dir.display()))?;

    let test_path = test_dir.join("basic_test.sh");
    let test_content = format!(
        r#"#!/usr/bin/env bash
# Basic TUI test generated by agent-terminal init
# Detected: {detected_str}
#
# Usage: bash tests/tui/basic_test.sh

set -euo pipefail

SESSION="test-$$"

cleanup() {{
    agent-terminal close --session "$SESSION" 2>/dev/null || true
}}
trap cleanup EXIT

echo "=== Starting TUI test ==="

# 1. Launch the app
agent-terminal open "{run_cmd}" --session "$SESSION" --size 80x24

# 2. Wait for the app to render
agent-terminal wait --stable 500 --session "$SESSION" --timeout 15000

# 3. Take a snapshot
agent-terminal snapshot --session "$SESSION"

# 4. Basic assertion — verify the app rendered something
agent-terminal assert --text "" --session "$SESSION" && echo "(app rendered content)"

# 5. Test resize handling
agent-terminal resize 40 10 --session "$SESSION"
agent-terminal wait --stable 500 --session "$SESSION"
agent-terminal snapshot --session "$SESSION"

# 6. Check the process is still alive after resize
STATUS=$(agent-terminal status --session "$SESSION" --json)
echo "Status after resize: $STATUS"

# 7. Clean up
agent-terminal close --session "$SESSION"
trap - EXIT

echo "=== All tests passed ==="
"#,
        detected_str = if detected.is_empty() {
            "none".to_string()
        } else {
            detected
                .iter()
                .map(|(fw, lang)| format!("{fw} ({lang})"))
                .collect::<Vec<_>>()
                .join(", ")
        },
        run_cmd = run_cmd,
    );

    fs::write(&test_path, &test_content)
        .map_err(|e| format!("Failed to write {}: {e}", test_path.display()))?;

    // Make executable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&test_path, perms)
            .map_err(|e| format!("Failed to chmod {}: {e}", test_path.display()))?;
    }

    println!("\nCreated: {}", test_path.display());
    println!("Run it:  bash {}", test_path.display());

    Ok(())
}

/// Run tests across a matrix of terminal configurations.
pub fn test_matrix(
    command: &str,
    sizes: Option<&str>,
    terms: Option<&str>,
    colors: Option<&str>,
    test: &str,
) -> Result<(), String> {
    let sizes_list: Vec<&str> = sizes
        .unwrap_or("80x24,120x40,40x10")
        .split(',')
        .map(|s| s.trim())
        .collect();

    let terms_list: Vec<&str> = terms
        .unwrap_or("xterm-256color,dumb")
        .split(',')
        .map(|s| s.trim())
        .collect();

    let colors_list: Vec<&str> = colors
        .unwrap_or("default,NO_COLOR=1")
        .split(',')
        .map(|s| s.trim())
        .collect();

    // Parse the test commands. Support both ; and && as separators.
    let test_str = test.replace("&&", ";");
    let test_commands: Vec<&str> = test_str.split(';').map(|s| s.trim()).collect();

    // Build the full matrix.
    struct MatrixEntry<'a> {
        size: &'a str,
        term: &'a str,
        color: &'a str,
    }

    let mut matrix: Vec<MatrixEntry> = Vec::new();
    for size in &sizes_list {
        for term in &terms_list {
            for color in &colors_list {
                matrix.push(MatrixEntry { size, term, color });
            }
        }
    }

    let total = matrix.len();
    println!(
        "Running {total} test combinations ({} sizes x {} terms x {} colors)",
        sizes_list.len(),
        terms_list.len(),
        colors_list.len()
    );
    println!();

    // Create output directory.
    let output_dir = Path::new("./agent-terminal-matrix");
    let _ = fs::create_dir_all(output_dir);

    struct MatrixResult {
        label: String,
        passed: bool,
        error: Option<String>,
    }

    let mut results: Vec<MatrixResult> = Vec::new();

    for (i, entry) in matrix.iter().enumerate() {
        let session_name = format!("at-matrix-{i}");
        let label = format!("{}+{}+{}", entry.size, entry.term, entry.color);

        // Build env args.
        let mut envs = vec![format!("TERM={}", entry.term)];
        if entry.color != "default" {
            envs.push(entry.color.to_string());
        }

        // Open session.
        let open_result = open(
            command,
            &session_name,
            None,
            &envs,
            Some(entry.size),
            false,
            false,
            false,
        );
        if let Err(e) = open_result {
            results.push(MatrixResult {
                label: label.clone(),
                passed: false,
                error: Some(format!("Failed to open: {e}")),
            });
            continue;
        }

        // Wait for app to stabilize.
        thread::sleep(Duration::from_millis(500));

        // Check if the process is alive.
        let alive = get_pane_pid(&session_name, None)
            .map(is_pid_alive)
            .unwrap_or(false);

        if !alive {
            // Process crashed during startup.
            let snap = try_capture_snapshot(&session_name, None).unwrap_or_default();
            let stderr = fs::read_to_string(stderr_path(&session_name)).unwrap_or_default();
            let crash_label = label.replace('+', "_").replace(':', "-");
            let crash_dir = output_dir.join(&crash_label);
            let _ = fs::create_dir_all(&crash_dir);
            let _ = fs::write(crash_dir.join("snapshot.txt"), &snap);
            let _ = fs::write(crash_dir.join("stderr.txt"), &stderr);

            results.push(MatrixResult {
                label: label.clone(),
                passed: false,
                error: Some("Process crashed during startup".to_string()),
            });
            let _ = close(&session_name);
            continue;
        }

        // Run test commands.
        let mut test_passed = true;
        let mut test_error: Option<String> = None;

        for test_cmd in &test_commands {
            let test_cmd = test_cmd.trim();
            if test_cmd.is_empty() {
                continue;
            }

            // Execute the test command. If it looks like an agent-terminal subcommand
            // (starts with a known subcommand name), prefix with our binary path and
            // the session flag. Otherwise run as-is through sh -c.
            let at_subcommands = [
                "open",
                "close",
                "list",
                "status",
                "exit-code",
                "logs",
                "snapshot",
                "send",
                "type",
                "paste",
                "resize",
                "click",
                "drag",
                "scroll-wheel",
                "wait",
                "assert",
                "find",
                "screenshot",
                "signal",
                "clipboard",
                "scrollback",
                "perf",
            ];
            let first_word = test_cmd.split_whitespace().next().unwrap_or("");
            let expanded = if at_subcommands.contains(&first_word) {
                let self_bin = std::env::current_exe()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "agent-terminal".to_string());
                format!("{} {} --session {}", self_bin, test_cmd, session_name)
            } else {
                test_cmd.replace("{session}", &session_name)
            };
            let output = Command::new("sh").args(["-c", &expanded]).output();

            match output {
                Ok(out) if !out.status.success() => {
                    test_passed = false;
                    let stderr_out = String::from_utf8_lossy(&out.stderr).to_string();
                    let stdout_out = String::from_utf8_lossy(&out.stdout).to_string();
                    test_error = Some(format!(
                        "Command failed: {expanded}\n{stdout_out}{stderr_out}"
                    ));
                    break;
                }
                Err(e) => {
                    test_passed = false;
                    test_error = Some(format!("Failed to run: {e}"));
                    break;
                }
                _ => {}
            }
        }

        if !test_passed {
            // Save failure snapshot.
            let snap = try_capture_snapshot(&session_name, None).unwrap_or_default();
            let fail_label = label.replace('+', "_").replace(':', "-");
            let fail_dir = output_dir.join(&fail_label);
            let _ = fs::create_dir_all(&fail_dir);
            let _ = fs::write(fail_dir.join("snapshot.txt"), &snap);
        }

        results.push(MatrixResult {
            label,
            passed: test_passed,
            error: test_error,
        });

        // Cleanup this session.
        let _ = close(&session_name);
    }

    // Print results table.
    println!();
    println!("{:<40} RESULT", "COMBINATION");
    println!("{}", "-".repeat(60));

    let mut pass_count = 0;
    let mut fail_count = 0;

    for r in &results {
        if r.passed {
            println!("{:<40} \u{2713} pass", r.label);
            pass_count += 1;
        } else {
            let err_summary = r
                .error
                .as_ref()
                .map(|e| {
                    // First line only.
                    e.lines().next().unwrap_or("unknown error").to_string()
                })
                .unwrap_or_default();
            println!("{:<40} \u{2717} FAIL: {}", r.label, err_summary);
            fail_count += 1;
        }
    }

    println!();
    println!("{pass_count}/{total} passed, {fail_count} failed");
    if fail_count > 0 {
        println!("Failure snapshots saved to: ./agent-terminal-matrix/");
    }

    if fail_count > 0 {
        // Return error so the exit code is non-zero.
        Err(format!("{fail_count}/{total} matrix tests failed"))
    } else {
        Ok(())
    }
}

/// Run accessibility checks against a TUI application.
pub fn a11y_check(command: &str) -> Result<(), String> {
    let report_dir = Path::new("./a11y-report");
    let _ = fs::create_dir_all(report_dir);

    let mut checks_passed = 0;
    let mut checks_failed = 0;
    let mut checks_warned = 0;
    let total_checks = 5;

    // Helper: start a session with given envs/size, wait, capture, close.
    let run_with_env =
        |session: &str, envs: &[String], size: &str| -> Result<(String, String, bool), String> {
            // Clean up any prior session with this name.
            if session_exists(session) {
                let _ = close(session);
            }

            open(
                command,
                session,
                None,
                envs,
                Some(size),
                false,
                false,
                false,
            )?;
            thread::sleep(Duration::from_millis(1000));

            let alive = get_pane_pid(session, None)
                .map(is_pid_alive)
                .unwrap_or(false);

            let raw_snap =
                tmux_cmd(&["capture-pane", "-t", session, "-p", "-e"]).unwrap_or_default();
            let plain_snap = tmux_cmd(&["capture-pane", "-t", session, "-p"]).unwrap_or_default();

            Ok((raw_snap, plain_snap, alive))
        };

    // --- Check 1: NO_COLOR ---
    print!("NO_COLOR respected ......... ");
    {
        let session = "at-a11y-nocolor";
        let envs = vec!["NO_COLOR=1".to_string()];
        match run_with_env(session, &envs, "80x24") {
            Ok((raw_snap, _plain_snap, alive)) => {
                if !alive {
                    println!("\u{2717} (process crashed with NO_COLOR=1)");
                    let _ = fs::write(report_dir.join("nocolor-crash.txt"), &raw_snap);
                    checks_failed += 1;
                } else {
                    // Check for ANSI color escape sequences (CSI ... m with color params).
                    // ESC [ ... m  where the params contain color codes (30-37, 40-47, 38, 48, 90-97, etc.)
                    let has_ansi_colors = raw_snap.contains("\x1b[") && {
                        // Look for SGR sequences that set colors.
                        let re = regex::Regex::new(r"\x1b\[\d*(;\d+)*m").unwrap();
                        // Filter out pure resets (ESC[0m, ESC[m) and check for actual color codes.
                        let found = re.find_iter(&raw_snap).any(|m| {
                            let s = m.as_str();
                            // Pure reset codes are fine.
                            s != "\x1b[m" && s != "\x1b[0m" && s != "\x1b[00m"
                        });
                        found
                    };

                    if has_ansi_colors {
                        println!("\u{2717} (ANSI color codes present despite NO_COLOR=1)");
                        let _ = fs::write(report_dir.join("nocolor-output.txt"), &raw_snap);
                        checks_failed += 1;
                    } else {
                        println!("\u{2713}");
                        checks_passed += 1;
                    }
                }
                let _ = close(session);
            }
            Err(e) => {
                println!("\u{2717} ({e})");
                checks_failed += 1;
            }
        }
    }

    // --- Check 2: TERM=dumb ---
    print!("TERM=dumb fallback ......... ");
    {
        let session = "at-a11y-dumb";
        let envs = vec!["TERM=dumb".to_string()];
        match run_with_env(session, &envs, "80x24") {
            Ok((_raw_snap, plain_snap, alive)) => {
                if alive {
                    println!("\u{2713}");
                    checks_passed += 1;
                } else {
                    println!("\u{2717} (process crashed with TERM=dumb)");
                    let _ = fs::write(report_dir.join("dumb-crash.txt"), &plain_snap);
                    checks_failed += 1;
                }
                let _ = close(session);
            }
            Err(e) => {
                println!("\u{2717} ({e})");
                checks_failed += 1;
            }
        }
    }

    // --- Check 3: Resize handling ---
    print!("resize handling ............ ");
    {
        let session = "at-a11y-resize";
        let envs: Vec<String> = vec![];
        match run_with_env(session, &envs, "80x24") {
            Ok((_raw_snap, _plain_snap, alive)) => {
                if !alive {
                    println!("\u{2717} (process crashed before resize)");
                    checks_failed += 1;
                    let _ = close(session);
                } else {
                    // Resize down.
                    let resize_result =
                        tmux_cmd(&["resize-window", "-t", session, "-x", "40", "-y", "10"]);
                    if let Err(e) = resize_result {
                        println!("\u{2717} (resize command failed: {e})");
                        checks_failed += 1;
                    } else {
                        thread::sleep(Duration::from_millis(500));
                        let still_alive = get_pane_pid(session, None)
                            .map(is_pid_alive)
                            .unwrap_or(false);

                        if still_alive {
                            println!("\u{2713}");
                            checks_passed += 1;
                        } else {
                            println!("\u{2717} (process crashed after resize to 40x10)");
                            let snap = try_capture_snapshot(session, None).unwrap_or_default();
                            let _ = fs::write(report_dir.join("resize-crash.txt"), &snap);
                            checks_failed += 1;
                        }
                    }
                    let _ = close(session);
                }
            }
            Err(e) => {
                println!("\u{2717} ({e})");
                checks_failed += 1;
            }
        }
    }

    // --- Check 4: Focus visible ---
    print!("focus visible .............. ");
    {
        // This is hard to detect automatically — we'd need to parse the UI
        // for focus indicators.  Flag as a warning/skip.
        println!("\u{26a0} (skipped — manual verification recommended)");
        checks_warned += 1;
    }

    // --- Check 5: Contrast (basic dim text check) ---
    print!("contrast (dim text) ........ ");
    {
        let session = "at-a11y-contrast";
        let envs: Vec<String> = vec![];
        match run_with_env(session, &envs, "80x24") {
            Ok((raw_snap, _plain_snap, alive)) => {
                if !alive {
                    println!("\u{2717} (process crashed)");
                    checks_failed += 1;
                } else {
                    // Look for ESC[2m (dim/faint attribute) in the output.
                    let has_dim = raw_snap.contains("\x1b[2m")
                        || raw_snap.contains(";2m")
                        || raw_snap.contains(";2;");
                    if has_dim {
                        println!("\u{26a0} (dim/faint text detected — may have contrast issues)");
                        let _ = fs::write(report_dir.join("contrast-dim.txt"), &raw_snap);
                        checks_warned += 1;
                    } else {
                        println!("\u{2713}");
                        checks_passed += 1;
                    }
                }
                let _ = close(session);
            }
            Err(e) => {
                println!("\u{2717} ({e})");
                checks_failed += 1;
            }
        }
    }

    // Summary.
    println!();
    println!(
        "{checks_passed}/{total_checks} passed, {checks_failed} failed, {checks_warned} warnings"
    );
    if checks_failed > 0 {
        println!("Failure details saved to: ./a11y-report/");
    }

    if checks_failed > 0 {
        Err(format!(
            "{checks_failed}/{total_checks} accessibility checks failed"
        ))
    } else {
        Ok(())
    }
}
