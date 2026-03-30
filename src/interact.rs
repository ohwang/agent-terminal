use std::process::Command;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tmux_cmd(args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run tmux: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        // Provide descriptive errors where possible.
        if stderr.contains("can't find") || stderr.contains("no server") {
            if let Some(session) = args.iter().find(|a| !a.starts_with('-')) {
                return Err(format!("Session '{}' not found", session));
            }
        }
        Err(format!("tmux error: {}", stderr))
    }
}

fn target_str(session: &str, pane: Option<&str>) -> String {
    match pane {
        Some(p) if p.starts_with('%') => p.to_string(),
        Some(p) => format!("{}:{}", session, p),
        None => session.to_string(),
    }
}

/// Build the tmux target string for passing to wait functions.
pub fn target_for_wait(session: &str, pane: Option<&str>) -> String {
    target_str(session, pane)
}

/// Map user-facing key names to their tmux equivalents.
fn map_key(key: &str) -> &str {
    match key {
        "PgUp" => "PageUp",
        "PgDn" => "PageDown",
        _ => key,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Send one or more named keys to a tmux pane.
///
/// Each key is mapped through `map_key` and sent individually via
/// `tmux send-keys`.  Prints nothing on success.
pub fn send_keys(keys: &[String], session: &str, pane: Option<&str>) -> Result<(), String> {
    let target = target_str(session, pane);
    for key in keys {
        let mapped = map_key(key);
        tmux_cmd(&["send-keys", "-t", &target, mapped])?;
    }
    Ok(())
}

/// Type literal text into the pane (no key-name interpretation).
pub fn type_text(text: &str, session: &str, pane: Option<&str>) -> Result<(), String> {
    let target = target_str(session, pane);
    tmux_cmd(&["send-keys", "-t", &target, "-l", text])?;
    Ok(())
}

/// Paste text via the tmux paste-buffer mechanism.
///
/// This safely handles special characters and multi-line text by first
/// loading the text into a tmux buffer and then pasting it.
pub fn paste(text: &str, session: &str, pane: Option<&str>) -> Result<(), String> {
    let target = target_str(session, pane);
    tmux_cmd(&["set-buffer", "--", text])?;
    tmux_cmd(&["paste-buffer", "-t", &target])?;
    Ok(())
}

/// Resize the terminal pane (and its parent window) to the given dimensions.
pub fn resize(cols: u16, rows: u16, session: &str, pane: Option<&str>) -> Result<(), String> {
    let target = target_str(session, pane);
    let cols_s = cols.to_string();
    let rows_s = rows.to_string();

    // Resize the window first so the pane has room.
    tmux_cmd(&["resize-window", "-t", session, "-x", &cols_s, "-y", &rows_s])?;
    tmux_cmd(&["resize-pane", "-t", &target, "-x", &cols_s, "-y", &rows_s])?;

    println!("{}x{}", cols, rows);
    Ok(())
}

/// Send an SGR mouse click (left or right, optionally double) to the pane.
///
/// Row and col are 1-indexed.  The SGR encoding is:
///   press:   `\x1b[<btn;col;rowM`
///   release: `\x1b[<btn;col;rowm`
/// where button 0 = left, button 2 = right.
pub fn click(row: u16, col: u16, session: &str, right: bool, double: bool) -> Result<(), String> {
    let target = target_str(session, None);
    let btn = if right { 2 } else { 0 };

    let press   = format!("\x1b[<{};{};{}M", btn, col, row);
    let release = format!("\x1b[<{};{};{}m", btn, col, row);

    let count = if double { 2 } else { 1 };
    for _ in 0..count {
        // Send press and release as separate commands so the target
        // application can read each escape sequence independently.
        tmux_cmd(&["send-keys", "-t", &target, "-l", &press])?;
        tmux_cmd(&["send-keys", "-t", &target, "-l", &release])?;
    }

    Ok(())
}

/// Send a mouse drag from (r1, c1) to (r2, c2).
///
/// Emits a button-press at the start position, then a button-release at the
/// end position.  Most terminal applications only inspect start/end so
/// intermediate motion events are omitted.
pub fn drag(r1: u16, c1: u16, r2: u16, c2: u16, session: &str) -> Result<(), String> {
    let target = target_str(session, None);

    // Button 0 = left; 32 = motion flag (added for move events).
    let press   = format!("\x1b[<0;{};{}M", c1, r1);
    let release = format!("\x1b[<0;{};{}m", c2, r2);

    // Send press and release as separate commands so the target
    // application can read each escape sequence independently.
    tmux_cmd(&["send-keys", "-t", &target, "-l", &press])?;
    tmux_cmd(&["send-keys", "-t", &target, "-l", &release])?;

    Ok(())
}

/// Send a scroll-wheel event at the given position.
///
/// SGR encoding: button 64 = scroll up, button 65 = scroll down.
pub fn scroll_wheel(direction: &str, row: u16, col: u16, session: &str) -> Result<(), String> {
    let target = target_str(session, None);

    let btn = match direction.to_lowercase().as_str() {
        "up" => 64,
        "down" => 65,
        other => return Err(format!("Unknown scroll direction '{}': use 'up' or 'down'", other)),
    };

    let seq = format!("\x1b[<{};{};{}M", btn, col, row);
    tmux_cmd(&["send-keys", "-t", &target, "-l", &seq])?;

    Ok(())
}

/// Send a real Unix signal to the process running inside the pane.
///
/// This is different from `send_keys "C-c"` which merely sends a keystroke.
pub fn signal(signal_name: &str, session: &str) -> Result<(), String> {
    // Obtain the PID of the foreground process in the pane.
    let pid_str = tmux_cmd(&[
        "display-message",
        "-t",
        session,
        "-p",
        "#{pane_pid}",
    ])?;
    let pid_str = pid_str.trim();
    let pid: i32 = pid_str
        .parse()
        .map_err(|_| format!("Failed to parse pane PID '{}' as integer", pid_str))?;

    let sig = parse_signal(signal_name)?;

    signal::kill(Pid::from_raw(pid), sig)
        .map_err(|e| format!("Failed to send {} to PID {}: {}", signal_name, pid, e))?;

    Ok(())
}

/// Clipboard operations: read, write, or paste the tmux paste buffer.
pub fn clipboard(operation: &str, text: Option<&str>, session: &str) -> Result<(), String> {
    match operation {
        "read" => {
            let buf = tmux_cmd(&["show-buffer"])?;
            print!("{}", buf);
            Ok(())
        }
        "write" => {
            let t = text.ok_or_else(|| "clipboard write requires text argument".to_string())?;
            tmux_cmd(&["set-buffer", "--", t])?;
            Ok(())
        }
        "paste" => {
            tmux_cmd(&["paste-buffer", "-t", session])?;
            Ok(())
        }
        other => Err(format!(
            "Unknown clipboard operation '{}': use 'read', 'write', or 'paste'",
            other
        )),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_signal(name: &str) -> Result<Signal, String> {
    // Accept with or without the SIG prefix, case-insensitive.
    let upper = name.to_uppercase();
    let canonical = if upper.starts_with("SIG") {
        upper.as_str()
    } else {
        // Temporary owned string — match below covers all branches.
        return match upper.as_str() {
            "INT" => Ok(Signal::SIGINT),
            "TERM" => Ok(Signal::SIGTERM),
            "WINCH" => Ok(Signal::SIGWINCH),
            "TSTP" => Ok(Signal::SIGTSTP),
            "CONT" => Ok(Signal::SIGCONT),
            "HUP" => Ok(Signal::SIGHUP),
            "KILL" => Ok(Signal::SIGKILL),
            "USR1" => Ok(Signal::SIGUSR1),
            "USR2" => Ok(Signal::SIGUSR2),
            _ => Err(format!("Unknown signal '{}'. Supported: SIGINT, SIGTERM, SIGWINCH, SIGTSTP, SIGCONT, SIGHUP, SIGKILL, SIGUSR1, SIGUSR2", name)),
        };
    };

    match canonical {
        "SIGINT" => Ok(Signal::SIGINT),
        "SIGTERM" => Ok(Signal::SIGTERM),
        "SIGWINCH" => Ok(Signal::SIGWINCH),
        "SIGTSTP" => Ok(Signal::SIGTSTP),
        "SIGCONT" => Ok(Signal::SIGCONT),
        "SIGHUP" => Ok(Signal::SIGHUP),
        "SIGKILL" => Ok(Signal::SIGKILL),
        "SIGUSR1" => Ok(Signal::SIGUSR1),
        "SIGUSR2" => Ok(Signal::SIGUSR2),
        _ => Err(format!(
            "Unknown signal '{}'. Supported: SIGINT, SIGTERM, SIGWINCH, SIGTSTP, SIGCONT, SIGHUP, SIGKILL, SIGUSR1, SIGUSR2",
            name
        )),
    }
}
