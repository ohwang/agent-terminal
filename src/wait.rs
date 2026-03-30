use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use regex::Regex;

use crate::ansi;

// ---------------------------------------------------------------------------
// Tmux helpers (inline until snapshot module is available)
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
        if stderr.contains("can't find") || stderr.contains("no server") {
            return Err(format!(
                "Session '{}' not found",
                args.iter()
                    .find(|a| !a.starts_with('-'))
                    .unwrap_or(&"unknown")
            ));
        }
        Err(format!("tmux error: {}", stderr))
    }
}

/// Capture plain text content of the pane (no ANSI escapes).
fn capture_plain(session: &str) -> Result<String, String> {
    tmux_cmd(&["capture-pane", "-t", session, "-p"])
}

/// Capture pane content with ANSI escape sequences preserved.
fn capture_ansi(session: &str) -> Result<String, String> {
    tmux_cmd(&["capture-pane", "-t", session, "-p", "-e"])
}

/// Get cursor position as (row, col), both 0-indexed from tmux.
fn get_cursor_position(session: &str) -> Result<(u64, u64), String> {
    let output = tmux_cmd(&[
        "display-message",
        "-t",
        session,
        "-p",
        "#{cursor_y} #{cursor_x}",
    ])?;
    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("Unexpected cursor output: '{}'", output.trim()));
    }
    let row: u64 = parts[0]
        .parse()
        .map_err(|_| format!("Failed to parse cursor row '{}'", parts[0]))?;
    let col: u64 = parts[1]
        .parse()
        .map_err(|_| format!("Failed to parse cursor col '{}'", parts[1]))?;
    Ok((row, col))
}

/// Get session diagnostic info for error messages.
fn session_diagnostics(session: &str) -> String {
    // Try to determine if the session is alive and get PID / runtime info.
    let pid_info = tmux_cmd(&[
        "display-message",
        "-t",
        session,
        "-p",
        "#{pane_pid} #{session_created}",
    ]);
    match pid_info {
        Ok(info) => {
            let parts: Vec<&str> = info.split_whitespace().collect();
            if parts.len() >= 2 {
                let pid = parts[0];
                let created: u64 = parts[1].parse().unwrap_or(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let runtime = now.saturating_sub(created);
                // Check if the pane's process is alive.
                let alive = Command::new("kill")
                    .args(["-0", pid])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                let status = if alive { "alive" } else { "dead" };
                format!(
                    "{} ({}, pid {}, runtime {}s)",
                    session, status, pid, runtime
                )
            } else {
                format!("{} (unknown state)", session)
            }
        }
        Err(_) => format!("{} (session not found)", session),
    }
}

/// Format a snapshot with numbered lines for display.
fn format_snapshot(content: &str) -> String {
    let mut out = String::new();
    for (i, line) in content.lines().enumerate() {
        out.push_str(&format!("  {}| {}\n", i + 1, line));
    }
    out
}

// ---------------------------------------------------------------------------
// Wait
// ---------------------------------------------------------------------------

/// Wait for screen stability and print the snapshot.
/// Used by `type --wait-stable` and `send --wait-stable`.
pub fn wait_stable_only(stable_ms: u64, session: &str) -> Result<(), String> {
    wait(
        None,
        None,
        None,
        Some(stable_ms),
        None,
        None,
        false,
        session,
        10_000,
        50,
    )
}

/// Poll-based wait system. Exactly one condition should be active.
///
/// Returns Ok(()) on success, Err with diagnostic message on timeout or error.
#[allow(clippy::too_many_arguments)]
pub fn wait(
    ms: Option<u64>,
    text: Option<&str>,
    text_gone: Option<&str>,
    stable: Option<u64>,
    cursor: Option<&str>,
    regex: Option<&str>,
    exit: bool,
    session: &str,
    timeout: u64,
    interval: u64,
) -> Result<(), String> {
    // 1. Hard wait
    if let Some(duration_ms) = ms {
        thread::sleep(Duration::from_millis(duration_ms));
        println!("Waited {}ms", duration_ms);
        return Ok(());
    }

    // 2. Wait for text to appear
    if let Some(target_text) = text {
        return wait_poll(
            session,
            timeout,
            interval,
            &format!("--text \"{}\"", target_text),
            |snapshot| {
                if snapshot.contains(target_text) {
                    Some(Ok(()))
                } else {
                    None
                }
            },
        );
    }

    // 3. Wait for text to disappear
    if let Some(gone_text) = text_gone {
        return wait_poll(
            session,
            timeout,
            interval,
            &format!("--text-gone \"{}\"", gone_text),
            |snapshot| {
                if !snapshot.contains(gone_text) {
                    Some(Ok(()))
                } else {
                    None
                }
            },
        );
    }

    // 4. Wait for screen stability
    if let Some(stable_ms) = stable {
        let mut last_snapshot = String::new();
        let mut last_change = Instant::now();
        let deadline = Instant::now() + Duration::from_millis(timeout);

        loop {
            let snapshot = capture_plain(session)?;

            if snapshot != last_snapshot {
                last_snapshot = snapshot;
                last_change = Instant::now();
            }

            let stable_duration = Instant::now().duration_since(last_change).as_millis() as u64;
            if stable_duration >= stable_ms {
                // Screen has been stable long enough.
                println!("{}", last_snapshot);
                return Ok(());
            }

            if Instant::now() >= deadline {
                return Err(format!(
                    "wait --stable {} timed out after {}ms\n\nSession: {}\nLast snapshot:\n{}Hint: Screen kept changing. Last stable for {}ms, needed {}ms.",
                    stable_ms,
                    timeout,
                    session_diagnostics(session),
                    format_snapshot(&last_snapshot),
                    stable_duration,
                    stable_ms,
                ));
            }

            thread::sleep(Duration::from_millis(interval));
        }
    }

    // 5. Wait for cursor position
    if let Some(cursor_str) = cursor {
        let parts: Vec<&str> = cursor_str.split(',').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid cursor format '{}': expected 'row,col'",
                cursor_str
            ));
        }
        let target_row: u64 = parts[0]
            .trim()
            .parse()
            .map_err(|_| format!("Invalid cursor row '{}'", parts[0]))?;
        let target_col: u64 = parts[1]
            .trim()
            .parse()
            .map_err(|_| format!("Invalid cursor col '{}'", parts[1]))?;

        let deadline = Instant::now() + Duration::from_millis(timeout);

        loop {
            let (row, col) = get_cursor_position(session)?;
            if row == target_row && col == target_col {
                let snapshot = capture_plain(session)?;
                println!("{}", snapshot);
                return Ok(());
            }

            if Instant::now() >= deadline {
                let last_snapshot = capture_plain(session).unwrap_or_default();
                return Err(format!(
                    "wait --cursor \"{},{}\" timed out after {}ms\n\nSession: {}\nCursor at: {},{}\nLast snapshot:\n{}Hint: Cursor is at row {}, col {} — expected row {}, col {}.",
                    target_row, target_col, timeout,
                    session_diagnostics(session),
                    row, col,
                    format_snapshot(&last_snapshot),
                    row, col, target_row, target_col,
                ));
            }

            thread::sleep(Duration::from_millis(interval));
        }
    }

    // 6. Wait for regex match
    if let Some(pattern) = regex {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?;

        return wait_poll(
            session,
            timeout,
            interval,
            &format!("--regex \"{}\"", pattern),
            |snapshot| {
                if re.is_match(snapshot) {
                    Some(Ok(()))
                } else {
                    None
                }
            },
        );
    }

    // 7. Wait for process exit
    if exit {
        let deadline = Instant::now() + Duration::from_millis(timeout);

        loop {
            // Check if the pane's process is still alive by querying tmux.
            let alive = tmux_cmd(&["display-message", "-t", session, "-p", "#{pane_dead}"]);

            match alive {
                Ok(output) => {
                    let trimmed = output.trim();
                    // #{pane_dead} returns "1" when the process has exited.
                    if trimmed == "1" {
                        println!("Process exited");
                        return Ok(());
                    }
                }
                Err(_) => {
                    // Session no longer exists — process exited and tmux cleaned up.
                    println!("Process exited (session gone)");
                    return Ok(());
                }
            }

            if Instant::now() >= deadline {
                return Err(format!(
                    "wait --exit timed out after {}ms — process is still running\n\nSession: {}",
                    timeout,
                    session_diagnostics(session),
                ));
            }

            thread::sleep(Duration::from_millis(interval));
        }
    }

    Err("No wait condition specified. Use one of: <ms>, --text, --text-gone, --stable, --cursor, --regex, --exit".to_string())
}

/// Generic poll loop for wait conditions that check snapshot content.
///
/// `check` receives the current snapshot and returns:
/// - `Some(Ok(()))` if the condition is met
/// - `Some(Err(msg))` if the condition definitively failed
/// - `None` if the condition is not yet met (keep polling)
fn wait_poll<F>(
    session: &str,
    timeout: u64,
    interval: u64,
    condition_desc: &str,
    check: F,
) -> Result<(), String>
where
    F: Fn(&str) -> Option<Result<(), String>>,
{
    let deadline = Instant::now() + Duration::from_millis(timeout);

    loop {
        let snapshot = capture_plain(session)?;

        match check(&snapshot) {
            Some(Ok(())) => {
                println!("{}", snapshot);
                return Ok(());
            }
            Some(Err(e)) => return Err(e),
            None => {}
        }

        if Instant::now() >= deadline {
            return Err(format!(
                "wait {} timed out after {}ms\n\nSession: {}\nLast snapshot:\n{}Hint: Condition was never satisfied within the timeout period.",
                condition_desc,
                timeout,
                session_diagnostics(session),
                format_snapshot(&snapshot),
            ));
        }

        thread::sleep(Duration::from_millis(interval));
    }
}

// ---------------------------------------------------------------------------
// Assert
// ---------------------------------------------------------------------------

/// Assertion commands. Each checks a condition and returns Ok(()) on pass,
/// Err on fail (which causes the CLI to exit with code 1).
#[allow(clippy::too_many_arguments)]
pub fn assert_cmd(
    text: Option<&str>,
    no_text: Option<&str>,
    row: Option<u16>,
    row_text: Option<&str>,
    cursor_row: Option<u16>,
    color: Option<u16>,
    color_style: Option<&str>,
    style: Option<&str>,
    style_check: Option<&str>,
    session: &str,
) -> Result<(), String> {
    // 1. Assert text is present
    if let Some(expected) = text {
        let snapshot = capture_plain(session)?;
        if snapshot.contains(expected) {
            println!("PASS: text \"{}\" found", expected);
            return Ok(());
        } else {
            return Err(format!(
                "FAIL: text \"{}\" not found\n\nSnapshot:\n{}",
                expected,
                format_snapshot(&snapshot),
            ));
        }
    }

    // 2. Assert text is absent
    if let Some(absent) = no_text {
        let snapshot = capture_plain(session)?;
        if !snapshot.contains(absent) {
            println!("PASS: text \"{}\" not found (as expected)", absent);
            return Ok(());
        } else {
            // Find where it occurs for context.
            let mut locations = Vec::new();
            for (i, line) in snapshot.lines().enumerate() {
                if let Some(col) = line.find(absent) {
                    locations.push(format!(
                        "  row {}, col {}: \"{}\"",
                        i + 1,
                        col + 1,
                        line.trim()
                    ));
                }
            }
            return Err(format!(
                "FAIL: text \"{}\" was found (expected absent)\n\nFound at:\n{}\n\nSnapshot:\n{}",
                absent,
                locations.join("\n"),
                format_snapshot(&snapshot),
            ));
        }
    }

    // 3. Assert row contains text
    if let Some(row_num) = row {
        let expected_text = row_text.ok_or_else(|| {
            "--row requires --row-text to specify what text to check for".to_string()
        })?;
        let snapshot = capture_plain(session)?;
        let lines: Vec<&str> = snapshot.lines().collect();
        let idx = (row_num as usize).saturating_sub(1);

        if idx >= lines.len() {
            return Err(format!(
                "FAIL: row {} does not exist (screen has {} rows)\n\nSnapshot:\n{}",
                row_num,
                lines.len(),
                format_snapshot(&snapshot),
            ));
        }

        let actual_line = lines[idx];
        if actual_line.contains(expected_text) {
            println!("PASS: row {} contains \"{}\"", row_num, expected_text);
            return Ok(());
        } else {
            return Err(format!(
                "FAIL: row {} does not contain \"{}\"\n\nRow {} actual content: \"{}\"\n\nSnapshot:\n{}",
                row_num,
                expected_text,
                row_num,
                actual_line,
                format_snapshot(&snapshot),
            ));
        }
    }

    // 4. Assert cursor on row
    if let Some(expected_row) = cursor_row {
        let (actual_row, _) = get_cursor_position(session)?;
        // tmux cursor_y is 0-indexed; user-facing rows are typically 0-indexed here too
        // since the cursor spec uses raw tmux values. We match the tmux convention.
        if actual_row == expected_row as u64 {
            println!("PASS: cursor on row {}", expected_row);
            return Ok(());
        } else {
            let snapshot = capture_plain(session)?;
            return Err(format!(
                "FAIL: cursor on row {} (expected row {})\n\nSnapshot:\n{}",
                actual_row,
                expected_row,
                format_snapshot(&snapshot),
            ));
        }
    }

    // 5. Assert color style on a row
    if let Some(row_num) = color {
        let expected_style_str = color_style.ok_or_else(|| {
            "--color requires --color-style to specify the expected style".to_string()
        })?;
        let ansi_content = capture_ansi(session)?;
        let lines: Vec<&str> = ansi_content.lines().collect();
        let idx = (row_num as usize).saturating_sub(1);

        if idx >= lines.len() {
            return Err(format!(
                "FAIL: row {} does not exist (screen has {} rows)",
                row_num,
                lines.len(),
            ));
        }

        let (plain, spans) = ansi::parse_ansi_line(lines[idx]);
        let required = ansi::parse_style_spec(expected_style_str);

        // Check if any span on this row matches the required style.
        let has_match = spans.iter().any(|s| {
            !plain[s.start..s.end].trim().is_empty() && ansi::style_matches(&s.style, &required)
        });

        if has_match {
            println!("PASS: row {} has style \"{}\"", row_num, expected_style_str);
            return Ok(());
        } else {
            let actual_styles: Vec<String> = spans
                .iter()
                .filter(|s| !plain[s.start..s.end].trim().is_empty())
                .map(|s| {
                    format!(
                        "  col {}: \"{}\" → {:?}",
                        s.start + 1,
                        &plain[s.start..s.end],
                        s.style
                    )
                })
                .collect();
            return Err(format!(
                "FAIL: row {} does not have style \"{}\"\n\nActual styles on row {}:\n{}",
                row_num,
                expected_style_str,
                row_num,
                actual_styles.join("\n"),
            ));
        }
    }

    // 6. Assert text has specific style
    if let Some(target_text) = style {
        let expected_style_str = style_check.ok_or_else(|| {
            "--style requires --style-check to specify the expected style".to_string()
        })?;
        let ansi_content = capture_ansi(session)?;
        let required = ansi::parse_style_spec(expected_style_str);

        // Search through all lines for the target text and check its style.
        for (line_idx, line) in ansi_content.lines().enumerate() {
            let (plain, spans) = ansi::parse_ansi_line(line);

            let mut search_start = 0;
            while let Some(pos) = plain[search_start..].find(target_text) {
                let abs_pos = search_start + pos;
                let text_end = abs_pos + target_text.len();

                // Find which spans cover this text range.
                let mut all_match = true;
                for span in &spans {
                    // Check if this span overlaps with our target text range.
                    if span.start < text_end
                        && span.end > abs_pos
                        && !ansi::style_matches(&span.style, &required)
                    {
                        all_match = false;
                        break;
                    }
                }

                if all_match {
                    println!(
                        "PASS: text \"{}\" has style \"{}\" (row {})",
                        target_text,
                        expected_style_str,
                        line_idx + 1
                    );
                    return Ok(());
                }

                search_start = abs_pos + 1;
            }
        }

        // Text not found or style doesn't match — provide context.
        let plain_snapshot = capture_plain(session)?;
        if plain_snapshot.contains(target_text) {
            return Err(format!(
                "FAIL: text \"{}\" found but does not have style \"{}\"\n\nSnapshot:\n{}",
                target_text,
                expected_style_str,
                format_snapshot(&plain_snapshot),
            ));
        } else {
            return Err(format!(
                "FAIL: text \"{}\" not found in screen\n\nSnapshot:\n{}",
                target_text,
                format_snapshot(&plain_snapshot),
            ));
        }
    }

    Err("No assertion specified. Use one of: --text, --no-text, --row/--row-text, --cursor-row, --color/--color-style, --style/--style-check".to_string())
}

// ---------------------------------------------------------------------------
// Find
// ---------------------------------------------------------------------------

/// A single match result for JSON output.
#[derive(serde::Serialize)]
struct FindMatch {
    row: usize,
    col: usize,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<ansi::Style>,
}

#[derive(serde::Serialize)]
struct FindResult {
    matches: Vec<FindMatch>,
}

/// Search the screen content for text, with optional regex and color filtering.
pub fn find(
    pattern: &str,
    all: bool,
    regex: bool,
    color: Option<&str>,
    json: bool,
    session: &str,
) -> Result<(), String> {
    // Color-based search requires ANSI capture
    if let Some(color_spec) = color {
        return find_by_color(pattern, color_spec, all, regex, json, session);
    }

    let snapshot = capture_plain(session)?;

    if regex {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?;

        let mut matches: Vec<FindMatch> = Vec::new();
        for (line_idx, line) in snapshot.lines().enumerate() {
            for mat in re.find_iter(line) {
                matches.push(FindMatch {
                    row: line_idx + 1,
                    col: mat.start() + 1,
                    text: mat.as_str().to_string(),
                    style: None,
                });
                if !all {
                    break;
                }
            }
            if !all && !matches.is_empty() {
                break;
            }
        }

        if matches.is_empty() {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&FindResult { matches }).unwrap()
                );
            }
            return Err(format!("Pattern /{}/ not found", pattern));
        }

        if json {
            let result = if all {
                matches
            } else {
                vec![matches.remove(0)]
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&FindResult { matches: result }).unwrap()
            );
        } else {
            if !all {
                println!(
                    "Found at row {}, col {}: \"{}\"",
                    matches[0].row,
                    matches[0].col,
                    extract_context_for_match(&snapshot, &matches[0])
                );
            } else {
                for m in &matches {
                    println!(
                        "row {}, col {}: \"{}\"",
                        m.row,
                        m.col,
                        extract_context_for_match(&snapshot, m)
                    );
                }
            }
        }
        return Ok(());
    }

    // Literal text search
    let mut matches: Vec<FindMatch> = Vec::new();
    for (line_idx, line) in snapshot.lines().enumerate() {
        let mut start = 0;
        while let Some(pos) = line[start..].find(pattern) {
            let abs_pos = start + pos;
            matches.push(FindMatch {
                row: line_idx + 1,
                col: abs_pos + 1,
                text: pattern.to_string(),
                style: None,
            });
            if !all {
                break;
            }
            start = abs_pos + 1;
        }
        if !all && !matches.is_empty() {
            break;
        }
    }

    if matches.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&FindResult { matches }).unwrap()
            );
        }
        return Err(format!("Text \"{}\" not found", pattern));
    }

    if json {
        let result = if all {
            matches
        } else {
            vec![matches.remove(0)]
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&FindResult { matches: result }).unwrap()
        );
    } else {
        if !all {
            println!(
                "Found at row {}, col {}: \"{}\"",
                matches[0].row,
                matches[0].col,
                extract_context_for_match(&snapshot, &matches[0])
            );
        } else {
            for m in &matches {
                println!(
                    "row {}, col {}: \"{}\"",
                    m.row,
                    m.col,
                    extract_context_for_match(&snapshot, m)
                );
            }
        }
    }
    Ok(())
}

/// Find text segments matching a color/style specification.
fn find_by_color(
    pattern: &str,
    color_spec: &str,
    all: bool,
    regex: bool,
    json: bool,
    session: &str,
) -> Result<(), String> {
    let ansi_content = capture_ansi(session)?;
    let required = ansi::parse_style_spec(color_spec);
    let re = if regex {
        Some(Regex::new(pattern).map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?)
    } else {
        None
    };

    let mut matches: Vec<FindMatch> = Vec::new();

    for (line_idx, line) in ansi_content.lines().enumerate() {
        let (plain, spans) = ansi::parse_ansi_line(line);

        for span in &spans {
            let text = &plain[span.start..span.end];
            if text.trim().is_empty() {
                continue;
            }
            if !ansi::style_matches(&span.style, &required) {
                continue;
            }

            let text_matches = if let Some(ref re) = re {
                re.is_match(text)
            } else {
                text.contains(pattern) || pattern.is_empty()
            };

            if text_matches {
                matches.push(FindMatch {
                    row: line_idx + 1,
                    col: span.start + 1,
                    text: text.trim().to_string(),
                    style: Some(span.style.clone()),
                });
                if !all {
                    break;
                }
            }
        }
        if !all && !matches.is_empty() {
            break;
        }
    }

    if matches.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&FindResult { matches }).unwrap()
            );
        }
        if pattern.is_empty() {
            return Err(format!("No text with style \"{}\" found", color_spec));
        }
        return Err(format!(
            "Text \"{}\" with style \"{}\" not found",
            pattern, color_spec
        ));
    }

    if json {
        let result = if all {
            matches
        } else {
            vec![matches.remove(0)]
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&FindResult { matches: result }).unwrap()
        );
    } else {
        if !all {
            println!(
                "Found at row {}, col {}: \"{}\"",
                matches[0].row, matches[0].col, matches[0].text
            );
        } else {
            for m in &matches {
                println!("row {}, col {}: \"{}\"", m.row, m.col, m.text);
            }
        }
    }
    Ok(())
}

/// Helper to get context string for a plain-text find match.
fn extract_context_for_match(snapshot: &str, m: &FindMatch) -> String {
    if let Some(line) = snapshot.lines().nth(m.row - 1) {
        let col = m.col - 1;
        extract_context(line, col, col + m.text.len())
    } else {
        m.text.clone()
    }
}

/// Extract context around a match for display.
fn extract_context(line: &str, match_start: usize, match_end: usize) -> String {
    let context_chars = 20;
    let start = match_start.saturating_sub(context_chars);
    let end = std::cmp::min(line.len(), match_end + context_chars);

    let mut result = String::new();
    if start > 0 {
        result.push_str("...");
    }
    result.push_str(&line[start..end]);
    if end < line.len() {
        result.push_str("...");
    }
    result
}
