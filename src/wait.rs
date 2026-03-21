use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use regex::Regex;

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
            return Err(format!("Session '{}' not found", args.iter().find(|a| !a.starts_with('-')).unwrap_or(&"unknown")));
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
        "display-message", "-t", session, "-p", "#{cursor_y} #{cursor_x}",
    ])?;
    let parts: Vec<&str> = output.trim().split_whitespace().collect();
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
        "display-message", "-t", session, "-p",
        "#{pane_pid} #{session_created}",
    ]);
    match pid_info {
        Ok(info) => {
            let parts: Vec<&str> = info.trim().split_whitespace().collect();
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
                format!("{} ({}, pid {}, runtime {}s)", session, status, pid, runtime)
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
// ANSI style parsing
// ---------------------------------------------------------------------------

/// A style descriptor parsed from ANSI SGR sequences.
#[derive(Debug, Clone, Default, PartialEq)]
struct AnsiStyle {
    fg: Option<String>,
    bg: Option<String>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    reverse: bool,
    strikethrough: bool,
}

/// A span of text with its associated style.
#[derive(Debug, Clone)]
struct StyledSpan {
    text: String,
    style: AnsiStyle,
    col: usize, // 0-indexed column where this span starts
}

/// Map a basic ANSI color code to a name.
fn color_name(code: u64) -> Option<String> {
    match code {
        0 => Some("black".to_string()),
        1 => Some("red".to_string()),
        2 => Some("green".to_string()),
        3 => Some("yellow".to_string()),
        4 => Some("blue".to_string()),
        5 => Some("magenta".to_string()),
        6 => Some("cyan".to_string()),
        7 => Some("white".to_string()),
        // Bright colors
        8 => Some("bright-black".to_string()),
        9 => Some("bright-red".to_string()),
        10 => Some("bright-green".to_string()),
        11 => Some("bright-yellow".to_string()),
        12 => Some("bright-blue".to_string()),
        13 => Some("bright-magenta".to_string()),
        14 => Some("bright-cyan".to_string()),
        15 => Some("bright-white".to_string()),
        _ => None,
    }
}

/// Apply SGR parameters to the current style.
fn apply_sgr(style: &mut AnsiStyle, params: &[u64]) {
    let mut i = 0;
    while i < params.len() {
        match params[i] {
            0 => *style = AnsiStyle::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            7 => style.reverse = true,
            9 => style.strikethrough = true,
            21 => style.underline = true, // double underline, treat as underline
            22 => { style.bold = false; style.dim = false; }
            23 => style.italic = false,
            24 => style.underline = false,
            27 => style.reverse = false,
            29 => style.strikethrough = false,
            // Foreground colors 30-37
            c @ 30..=37 => style.fg = color_name(c - 30),
            // Extended foreground: 38;5;N or 38;2;R;G;B
            38 => {
                if i + 1 < params.len() {
                    if params[i + 1] == 5 && i + 2 < params.len() {
                        // 256-color mode
                        let c = params[i + 2];
                        style.fg = color_name(c).or_else(|| Some(format!("{}", c)));
                        i += 2;
                    } else if params[i + 1] == 2 && i + 4 < params.len() {
                        // True color
                        style.fg = Some(format!("#{:02x}{:02x}{:02x}",
                            params[i + 2], params[i + 3], params[i + 4]));
                        i += 4;
                    }
                }
            }
            39 => style.fg = None,
            // Background colors 40-47
            c @ 40..=47 => style.bg = color_name(c - 40),
            // Extended background: 48;5;N or 48;2;R;G;B
            48 => {
                if i + 1 < params.len() {
                    if params[i + 1] == 5 && i + 2 < params.len() {
                        let c = params[i + 2];
                        style.bg = color_name(c).or_else(|| Some(format!("{}", c)));
                        i += 2;
                    } else if params[i + 1] == 2 && i + 4 < params.len() {
                        style.bg = Some(format!("#{:02x}{:02x}{:02x}",
                            params[i + 2], params[i + 3], params[i + 4]));
                        i += 4;
                    }
                }
            }
            49 => style.bg = None,
            // Bright foreground colors 90-97
            c @ 90..=97 => style.fg = color_name(c - 90 + 8),
            // Bright background colors 100-107
            c @ 100..=107 => style.bg = color_name(c - 100 + 8),
            _ => {} // Ignore unknown codes
        }
        i += 1;
    }
}

/// Parse a single line of ANSI-escaped text into styled spans.
fn parse_ansi_line(line: &str) -> Vec<StyledSpan> {
    let mut spans = Vec::new();
    let mut style = AnsiStyle::default();
    let mut current_text = String::new();
    let mut col: usize = 0;
    let mut span_start_col: usize = 0;

    let bytes = line.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            // Save any accumulated text as a span.
            if !current_text.is_empty() {
                spans.push(StyledSpan {
                    text: current_text.clone(),
                    style: style.clone(),
                    col: span_start_col,
                });
                current_text.clear();
            }

            // Parse the CSI sequence: ESC [ params final_byte
            i += 2; // skip ESC [
            let mut param_str = String::new();
            while i < bytes.len() && bytes[i] != b'm' && !(bytes[i] >= 0x40 && bytes[i] <= 0x7e) {
                param_str.push(bytes[i] as char);
                i += 1;
            }

            if i < bytes.len() {
                let final_byte = bytes[i] as char;
                i += 1; // skip the final byte

                if final_byte == 'm' {
                    // SGR sequence
                    let params: Vec<u64> = if param_str.is_empty() {
                        vec![0] // ESC[m is the same as ESC[0m (reset)
                    } else {
                        param_str
                            .split(';')
                            .map(|s| s.parse::<u64>().unwrap_or(0))
                            .collect()
                    };
                    apply_sgr(&mut style, &params);
                    span_start_col = col;
                }
                // Ignore other CSI sequences (cursor movement, etc.)
            }
        } else {
            if current_text.is_empty() {
                span_start_col = col;
            }
            current_text.push(bytes[i] as char);
            col += 1;
            i += 1;
        }
    }

    // Push any remaining text.
    if !current_text.is_empty() {
        spans.push(StyledSpan {
            text: current_text,
            style: style.clone(),
            col: span_start_col,
        });
    }

    spans
}

/// Parse a style specification string like "fg:red,bold,underline" into an AnsiStyle.
fn parse_style_spec(spec: &str) -> AnsiStyle {
    let mut style = AnsiStyle::default();
    for part in spec.split(',') {
        let part = part.trim();
        if let Some(color) = part.strip_prefix("fg:") {
            style.fg = Some(color.to_lowercase());
        } else if let Some(color) = part.strip_prefix("bg:") {
            style.bg = Some(color.to_lowercase());
        } else {
            match part.to_lowercase().as_str() {
                "bold" => style.bold = true,
                "dim" => style.dim = true,
                "italic" => style.italic = true,
                "underline" => style.underline = true,
                "reverse" => style.reverse = true,
                "strikethrough" => style.strikethrough = true,
                _ => {} // Ignore unknown style parts
            }
        }
    }
    style
}

/// Check if an actual style matches the required style spec.
/// Only checks the attributes that are set in the spec (non-default).
fn style_matches(actual: &AnsiStyle, required: &AnsiStyle) -> bool {
    if required.fg.is_some() && actual.fg != required.fg {
        return false;
    }
    if required.bg.is_some() && actual.bg != required.bg {
        return false;
    }
    if required.bold && !actual.bold {
        return false;
    }
    if required.dim && !actual.dim {
        return false;
    }
    if required.italic && !actual.italic {
        return false;
    }
    if required.underline && !actual.underline {
        return false;
    }
    if required.reverse && !actual.reverse {
        return false;
    }
    if required.strikethrough && !actual.strikethrough {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// Wait
// ---------------------------------------------------------------------------

/// Poll-based wait system. Exactly one condition should be active.
///
/// Returns Ok(()) on success, Err with diagnostic message on timeout or error.
pub fn wait(
    ms: Option<u64>,
    text: Option<&str>,
    text_gone: Option<&str>,
    stable: Option<u64>,
    cursor: Option<&str>,
    regex: Option<&str>,
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
        return wait_poll(session, timeout, interval, &format!("--text \"{}\"", target_text), |snapshot| {
            if snapshot.contains(target_text) {
                Some(Ok(()))
            } else {
                None
            }
        });
    }

    // 3. Wait for text to disappear
    if let Some(gone_text) = text_gone {
        return wait_poll(session, timeout, interval, &format!("--text-gone \"{}\"", gone_text), |snapshot| {
            if !snapshot.contains(gone_text) {
                Some(Ok(()))
            } else {
                None
            }
        });
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
        let re = Regex::new(pattern)
            .map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?;

        return wait_poll(session, timeout, interval, &format!("--regex \"{}\"", pattern), |snapshot| {
            if re.is_match(snapshot) {
                Some(Ok(()))
            } else {
                None
            }
        });
    }

    Err("No wait condition specified. Use one of: <ms>, --text, --text-gone, --stable, --cursor, --regex".to_string())
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
                    locations.push(format!("  row {}, col {}: \"{}\"", i + 1, col + 1, line.trim()));
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

        let spans = parse_ansi_line(lines[idx]);
        let required = parse_style_spec(expected_style_str);

        // Check if any span on this row matches the required style.
        let has_match = spans.iter().any(|s| !s.text.trim().is_empty() && style_matches(&s.style, &required));

        if has_match {
            println!("PASS: row {} has style \"{}\"", row_num, expected_style_str);
            return Ok(());
        } else {
            let actual_styles: Vec<String> = spans
                .iter()
                .filter(|s| !s.text.trim().is_empty())
                .map(|s| format!("  col {}: \"{}\" → {:?}", s.col + 1, s.text, s.style))
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
        let required = parse_style_spec(expected_style_str);

        // Search through all lines for the target text and check its style.
        for (line_idx, line) in ansi_content.lines().enumerate() {
            let spans = parse_ansi_line(line);

            // Reconstruct the plain text from spans to find the target text,
            // then determine which spans cover it and check their styles.
            let plain: String = spans.iter().map(|s| s.text.as_str()).collect();

            let mut search_start = 0;
            while let Some(pos) = plain[search_start..].find(target_text) {
                let abs_pos = search_start + pos;
                let text_end = abs_pos + target_text.len();

                // Find which spans cover this text range.
                let mut all_match = true;
                let mut char_pos = 0;
                for span in &spans {
                    let span_start = char_pos;
                    let span_end = char_pos + span.text.len();

                    // Check if this span overlaps with our target text range.
                    if span_start < text_end && span_end > abs_pos {
                        if !style_matches(&span.style, &required) {
                            all_match = false;
                            break;
                        }
                    }
                    char_pos = span_end;
                }

                if all_match {
                    println!(
                        "PASS: text \"{}\" has style \"{}\" (row {})",
                        target_text, expected_style_str, line_idx + 1
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

/// Search the screen content for text, with optional regex and color filtering.
pub fn find(
    pattern: &str,
    all: bool,
    regex: bool,
    color: Option<&str>,
    session: &str,
) -> Result<(), String> {
    // Color-based search requires ANSI capture
    if let Some(color_spec) = color {
        return find_by_color(pattern, color_spec, all, regex, session);
    }

    let snapshot = capture_plain(session)?;

    if regex {
        let re = Regex::new(pattern)
            .map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?;

        let mut matches = Vec::new();
        for (line_idx, line) in snapshot.lines().enumerate() {
            for mat in re.find_iter(line) {
                let context = extract_context(line, mat.start(), mat.end());
                matches.push(format!(
                    "row {}, col {}: \"{}\"",
                    line_idx + 1,
                    mat.start() + 1,
                    context,
                ));
                if !all {
                    println!("Found at {}", matches[0]);
                    return Ok(());
                }
            }
        }

        if matches.is_empty() {
            return Err(format!("Pattern /{}/ not found", pattern));
        }
        for m in &matches {
            println!("{}", m);
        }
        return Ok(());
    }

    // Literal text search
    let mut matches = Vec::new();
    for (line_idx, line) in snapshot.lines().enumerate() {
        let mut start = 0;
        while let Some(pos) = line[start..].find(pattern) {
            let abs_pos = start + pos;
            let context = extract_context(line, abs_pos, abs_pos + pattern.len());
            matches.push(format!(
                "row {}, col {}: \"{}\"",
                line_idx + 1,
                abs_pos + 1,
                context,
            ));
            if !all {
                println!("Found at {}", matches[0]);
                return Ok(());
            }
            start = abs_pos + 1;
        }
    }

    if matches.is_empty() {
        return Err(format!("Text \"{}\" not found", pattern));
    }
    for m in &matches {
        println!("{}", m);
    }
    Ok(())
}

/// Find text segments matching a color/style specification.
fn find_by_color(
    pattern: &str,
    color_spec: &str,
    all: bool,
    regex: bool,
    session: &str,
) -> Result<(), String> {
    let ansi_content = capture_ansi(session)?;
    let required = parse_style_spec(color_spec);
    let re = if regex {
        Some(Regex::new(pattern).map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?)
    } else {
        None
    };

    let mut matches = Vec::new();

    for (line_idx, line) in ansi_content.lines().enumerate() {
        let spans = parse_ansi_line(line);

        for span in &spans {
            if span.text.trim().is_empty() {
                continue;
            }
            if !style_matches(&span.style, &required) {
                continue;
            }

            // Check if the span text matches the pattern.
            let text_matches = if let Some(ref re) = re {
                re.is_match(&span.text)
            } else {
                span.text.contains(pattern) || pattern.is_empty()
            };

            if text_matches {
                matches.push(format!(
                    "row {}, col {}: \"{}\"",
                    line_idx + 1,
                    span.col + 1,
                    span.text.trim(),
                ));
                if !all {
                    println!("Found at {}", matches[0]);
                    return Ok(());
                }
            }
        }
    }

    if matches.is_empty() {
        if pattern.is_empty() {
            return Err(format!("No text with style \"{}\" found", color_spec));
        }
        return Err(format!(
            "Text \"{}\" with style \"{}\" not found",
            pattern, color_spec
        ));
    }
    for m in &matches {
        println!("{}", m);
    }
    Ok(())
}

/// Extract context around a match for display.
fn extract_context(line: &str, match_start: usize, match_end: usize) -> String {
    let context_chars = 20;
    let start = if match_start > context_chars {
        match_start - context_chars
    } else {
        0
    };
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
