use serde::Serialize;
use std::process::Command;

pub use crate::ansi::{dominant_style, parse_ansi, parse_ansi_line, Line, Span, Style};

#[derive(Serialize)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Serialize)]
pub struct Cursor {
    pub row: u16,
    pub col: u16,
}

#[derive(Serialize)]
pub struct JsonSnapshot {
    pub session: String,
    pub size: Size,
    pub cursor: Cursor,
    pub lines: Vec<Line>,
}

// ---------------------------------------------------------------------------
// Pane layout types and queries
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct PaneLayout {
    pub pane_id: String,
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,
    pub title: String,
    pub active: bool,
}

/// Query the layout of all panes in the active window of a session.
pub fn list_pane_layouts(session: &str) -> Result<Vec<PaneLayout>, String> {
    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-t",
            session,
            "-F",
            "#{pane_id}\t#{pane_left}\t#{pane_top}\t#{pane_width}\t#{pane_height}\t#{pane_title}\t#{pane_active}",
        ])
        .output()
        .map_err(|e| format!("Failed to run tmux list-panes: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "tmux list-panes failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut panes = Vec::new();
    for line in stdout.trim().lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 7 {
            continue;
        }
        panes.push(PaneLayout {
            pane_id: parts[0].to_string(),
            left: parts[1].parse().unwrap_or(0),
            top: parts[2].parse().unwrap_or(0),
            width: parts[3].parse().unwrap_or(0),
            height: parts[4].parse().unwrap_or(0),
            title: parts[5].to_string(),
            active: parts[6] == "1",
        });
    }
    Ok(panes)
}

/// Get the total window size (cols, rows) for a session.
pub fn get_window_size(session: &str) -> Result<(u16, u16), String> {
    let output = Command::new("tmux")
        .args([
            "display-message",
            "-t",
            session,
            "-p",
            "#{window_width} #{window_height}",
        ])
        .output()
        .map_err(|e| format!("Failed to run tmux display-message: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "tmux display-message failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(format!(
            "Unexpected window size output: {:?}",
            stdout.trim()
        ));
    }
    let cols = parts[0]
        .parse::<u16>()
        .map_err(|e| format!("Bad window_width: {}", e))?;
    let rows = parts[1]
        .parse::<u16>()
        .map_err(|e| format!("Bad window_height: {}", e))?;
    Ok((cols, rows))
}

// ---------------------------------------------------------------------------
// tmux helpers
// ---------------------------------------------------------------------------

fn target_str(session: &str, pane: Option<&str>) -> String {
    match pane {
        // Global pane IDs (%N) are already unique — use directly
        Some(p) if p.starts_with('%') => p.to_string(),
        Some(p) => format!("{}:{}", session, p),
        None => session.to_string(),
    }
}

/// Fetch pane geometry and cursor position.
/// Returns (cols, rows, cursor_x, cursor_y).
pub fn get_pane_info(session: &str, pane: Option<&str>) -> Result<(u16, u16, u16, u16), String> {
    let target = target_str(session, pane);
    let output = Command::new("tmux")
        .args([
            "display-message",
            "-t",
            &target,
            "-p",
            "#{pane_width} #{pane_height} #{cursor_x} #{cursor_y}",
        ])
        .output()
        .map_err(|e| format!("Failed to run tmux display-message: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "tmux display-message failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(format!(
            "Unexpected tmux display-message output: {:?}",
            stdout.trim()
        ));
    }

    let cols = parts[0]
        .parse::<u16>()
        .map_err(|e| format!("Bad pane_width: {}", e))?;
    let rows = parts[1]
        .parse::<u16>()
        .map_err(|e| format!("Bad pane_height: {}", e))?;
    let cx = parts[2]
        .parse::<u16>()
        .map_err(|e| format!("Bad cursor_x: {}", e))?;
    let cy = parts[3]
        .parse::<u16>()
        .map_err(|e| format!("Bad cursor_y: {}", e))?;

    Ok((cols, rows, cx, cy))
}

/// Capture plain text (no ANSI escapes) from the pane.
pub fn capture_plain(session: &str, pane: Option<&str>) -> Result<String, String> {
    let target = target_str(session, pane);
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", &target, "-p"])
        .output()
        .map_err(|e| format!("Failed to run tmux capture-pane: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "tmux capture-pane failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Capture with ANSI escape sequences preserved.
pub fn capture_ansi(session: &str, pane: Option<&str>) -> Result<String, String> {
    let target = target_str(session, pane);
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", &target, "-e", "-p"])
        .output()
        .map_err(|e| format!("Failed to run tmux capture-pane: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "tmux capture-pane failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Capture with scrollback history.
fn capture_with_scrollback(
    session: &str,
    pane: Option<&str>,
    lines: usize,
    with_ansi: bool,
) -> Result<String, String> {
    let target = target_str(session, pane);
    let scroll_arg = format!("-{}", lines);
    let mut args = vec!["capture-pane", "-t", &target, "-p", "-S", &scroll_arg];
    if with_ansi {
        args.insert(4, "-e"); // before -p
    }
    let output = Command::new("tmux")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run tmux capture-pane: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "tmux capture-pane failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

fn format_header(cols: u16, rows: u16, cx: u16, cy: u16, session: &str) -> String {
    format!(
        "[size: {}x{}  cursor: {},{}  session: {}]",
        cols, rows, cx, cy, session
    )
}

fn separator_line(width: usize) -> String {
    "\u{2500}".repeat(width.max(45))
}

/// Trim trailing empty lines from a list of lines.
fn trim_trailing_empty<'a>(lines: &'a [&'a str]) -> &'a [&'a str] {
    let mut end = lines.len();
    while end > 0 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    &lines[..end]
}

/// Width needed for line number column given total number of lines.
fn line_number_width(total: usize) -> usize {
    if total == 0 {
        return 1;
    }
    let digits = ((total as f64).log10().floor() as usize) + 1;
    digits.max(2)
}

// ---------------------------------------------------------------------------
// Output modes
// ---------------------------------------------------------------------------

fn output_plain(content: &str, cols: u16, rows: u16, cx: u16, cy: u16, session: &str) {
    let header = format_header(cols, rows, cx, cy, session);
    println!("{}", header);
    println!("{}", separator_line(header.len()));

    let all_lines: Vec<&str> = content.lines().collect();
    let lines = trim_trailing_empty(&all_lines);
    let width = line_number_width(lines.len());

    for (i, line) in lines.iter().enumerate() {
        println!("{:>width$}\u{2502} {}", i + 1, line, width = width);
    }
}

fn output_color(ansi_content: &str, cols: u16, rows: u16, cx: u16, cy: u16, session: &str) {
    let header = format_header(cols, rows, cx, cy, session);
    println!("{}", header);
    println!("{}", separator_line(header.len()));

    let all_lines: Vec<&str> = ansi_content.lines().collect();
    let lines = trim_trailing_empty(&all_lines);
    let width = line_number_width(lines.len());

    for (i, raw_line) in lines.iter().enumerate() {
        let segments = parse_ansi(raw_line);
        let plain: String = segments.iter().map(|(t, _)| t.as_str()).collect();
        let dom = dominant_style(&segments);
        let annotation = if dom.is_default() {
            String::new()
        } else {
            format!("  {}", dom.annotation())
        };
        println!(
            "{:>width$}\u{2502} {}{}",
            i + 1,
            plain,
            annotation,
            width = width,
        );
    }
}

fn output_raw(ansi_content: &str) {
    print!("{}", ansi_content);
}

fn output_ansi(ansi_content: &str, cols: u16, rows: u16, cx: u16, cy: u16, session: &str) {
    let header = format_header(cols, rows, cx, cy, session);
    println!("{}", header);
    println!("{}", separator_line(header.len()));

    let all_lines: Vec<&str> = ansi_content.lines().collect();
    let lines = trim_trailing_empty(&all_lines);
    let width = line_number_width(lines.len());

    for (i, line) in lines.iter().enumerate() {
        println!("{:>width$}\u{2502} {}", i + 1, line, width = width);
    }
}

fn output_json(
    ansi_content: &str,
    cols: u16,
    rows: u16,
    cx: u16,
    cy: u16,
    session: &str,
) -> Result<(), String> {
    let all_lines: Vec<&str> = ansi_content.lines().collect();
    let lines = trim_trailing_empty(&all_lines);

    let json_lines: Vec<Line> = lines
        .iter()
        .enumerate()
        .map(|(i, raw_line)| {
            let (text, spans) = parse_ansi_line(raw_line);
            Line {
                row: i + 1,
                text,
                spans,
            }
        })
        .collect();

    let snapshot = JsonSnapshot {
        session: session.to_string(),
        size: Size { cols, rows },
        cursor: Cursor { row: cy, col: cx },
        lines: json_lines,
    };

    let json = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| format!("JSON serialization failed: {}", e))?;
    println!("{}", json);
    Ok(())
}

fn output_diff(content: &str, session: &str, cols: u16, rows: u16, cx: u16, cy: u16) {
    let snapshot_path = format!("/tmp/agent-terminal-{}-last-snapshot", session);
    let header = format_header(cols, rows, cx, cy, session);
    println!("{}", header);
    println!("{}", separator_line(header.len()));

    let current_lines: Vec<&str> = content.lines().collect();
    let current_lines = trim_trailing_empty(&current_lines);

    let prev_content = std::fs::read_to_string(&snapshot_path).unwrap_or_default();
    let prev_lines: Vec<&str> = prev_content.lines().collect();
    let prev_lines_trimmed = trim_trailing_empty(&prev_lines);

    let max_lines = current_lines.len().max(prev_lines_trimmed.len());
    let width = line_number_width(max_lines);
    let mut any_diff = false;

    for i in 0..max_lines {
        let cur = current_lines.get(i).copied().unwrap_or("");
        let prev = prev_lines_trimmed.get(i).copied().unwrap_or("");

        if cur != prev {
            any_diff = true;
            if !prev.is_empty() {
                println!("-{:>width$}\u{2502} {}", i + 1, prev, width = width,);
            }
            println!("+{:>width$}\u{2502} {}", i + 1, cur, width = width,);
        } else {
            println!(" {:>width$}\u{2502} {}", i + 1, cur, width = width,);
        }
    }

    if !any_diff {
        println!("(no changes)");
    }

    // Store current snapshot for next diff
    let _ = std::fs::write(&snapshot_path, content);
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn snapshot(
    session: &str,
    pane: Option<&str>,
    window: bool,
    color: bool,
    raw: bool,
    ansi: bool,
    json: bool,
    diff: bool,
    scrollback: Option<usize>,
) -> Result<(), String> {
    if window {
        return snapshot_window(session, json, color, ansi, raw);
    }

    // --raw: direct pass-through, no pane info needed
    if raw {
        let content = match scrollback {
            Some(n) => capture_with_scrollback(session, pane, n, true)?,
            None => capture_ansi(session, pane)?,
        };
        output_raw(&content);
        return Ok(());
    }

    let (cols, rows, cx, cy) = get_pane_info(session, pane)?;

    if json || color || ansi {
        // These modes need ANSI escape data
        let ansi_content = match scrollback {
            Some(n) => capture_with_scrollback(session, pane, n, true)?,
            None => capture_ansi(session, pane)?,
        };

        if json {
            return output_json(&ansi_content, cols, rows, cx, cy, session);
        } else if color {
            output_color(&ansi_content, cols, rows, cx, cy, session);
        } else {
            // --ansi
            output_ansi(&ansi_content, cols, rows, cx, cy, session);
        }
    } else {
        // Plain text or diff
        let content = match scrollback {
            Some(n) => capture_with_scrollback(session, pane, n, false)?,
            None => capture_plain(session, pane)?,
        };

        if diff {
            output_diff(&content, session, cols, rows, cx, cy);
        } else {
            output_plain(&content, cols, rows, cx, cy, session);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Window-level snapshot (all panes composited)
// ---------------------------------------------------------------------------

fn snapshot_window(
    session: &str,
    json: bool,
    color: bool,
    ansi: bool,
    raw: bool,
) -> Result<(), String> {
    let panes = list_pane_layouts(session)?;
    let (win_cols, win_rows) = get_window_size(session)?;

    if panes.len() == 1 {
        // Single pane — delegate to normal snapshot (active pane)
        return snapshot(session, None, false, color, raw, ansi, json, false, None);
    }

    if json {
        return output_window_json(session, &panes, win_cols, win_rows);
    }

    let use_ansi = color || ansi;

    // Build a 2D grid of the full window
    let mut grid: Vec<Vec<char>> = vec![vec![' '; win_cols as usize]; win_rows as usize];

    // Fill separator positions with border characters
    // First mark all cells, then overlay pane content
    let pane_mask: Vec<Vec<bool>> = {
        let mut mask = vec![vec![false; win_cols as usize]; win_rows as usize];
        for p in &panes {
            for row in p.top..(p.top + p.height).min(win_rows) {
                for col in p.left..(p.left + p.width).min(win_cols) {
                    mask[row as usize][col as usize] = true;
                }
            }
        }
        mask
    };

    // Fill non-pane cells with separator chars
    for row in 0..win_rows as usize {
        for col in 0..win_cols as usize {
            if !pane_mask[row][col] {
                // Determine if this is a vertical or horizontal separator
                let has_left = col > 0 && pane_mask[row][col - 1];
                let has_right = col + 1 < win_cols as usize && pane_mask[row][col + 1];
                if has_left || has_right {
                    grid[row][col] = '\u{2502}'; // │
                } else {
                    grid[row][col] = '\u{2500}'; // ─
                }
            }
        }
    }

    if use_ansi {
        // Capture ANSI content per pane and composite with escape sequences
        let mut ansi_grid: Vec<Vec<String>> = grid
            .iter()
            .map(|row| row.iter().map(|c| c.to_string()).collect())
            .collect();

        for p in &panes {
            let content = capture_ansi(session, Some(&p.pane_id))?;
            let lines: Vec<&str> = content.lines().collect();
            for (line_idx, line) in lines.iter().enumerate() {
                let row = p.top as usize + line_idx;
                if row >= win_rows as usize {
                    break;
                }
                // Parse the ANSI line and place each character-cell
                let mut col_offset = 0usize;
                let mut current_sgr = String::new();
                let mut chars = line.chars().peekable();
                while let Some(ch) = chars.next() {
                    if ch == '\x1b' {
                        // Accumulate the escape sequence
                        let mut seq = String::from('\x1b');
                        if chars.peek() == Some(&'[') {
                            seq.push(chars.next().unwrap());
                            while let Some(&c) = chars.peek() {
                                seq.push(chars.next().unwrap());
                                if c.is_ascii_alphabetic() {
                                    break;
                                }
                            }
                        }
                        current_sgr.push_str(&seq);
                    } else {
                        let col = p.left as usize + col_offset;
                        if col < win_cols as usize {
                            let cell = if current_sgr.is_empty() {
                                ch.to_string()
                            } else {
                                let s = format!("{}{}", current_sgr, ch);
                                current_sgr.clear();
                                s
                            };
                            ansi_grid[row][col] = cell;
                        }
                        col_offset += 1;
                    }
                }
            }
        }

        // Output
        let header = format!(
            "[window: {}x{}  panes: {}  session: {}]",
            win_cols,
            win_rows,
            panes.len(),
            session
        );
        println!("{}", header);
        println!("{}", separator_line(header.len()));

        let width = line_number_width(win_rows as usize);
        for (i, row) in ansi_grid.iter().enumerate() {
            let line: String = row.concat();
            let line = format!("{}\x1b[0m", line); // reset at end of each line
            println!("{:>width$}\u{2502} {}", i + 1, line, width = width);
        }
    } else {
        // Plain text: capture each pane and place into the grid
        for p in &panes {
            let content = capture_plain(session, Some(&p.pane_id))?;
            let lines: Vec<&str> = content.lines().collect();
            for (line_idx, line) in lines.iter().enumerate() {
                let row = p.top as usize + line_idx;
                if row >= win_rows as usize {
                    break;
                }
                for (col_offset, ch) in line.chars().enumerate() {
                    let col = p.left as usize + col_offset;
                    if col < win_cols as usize {
                        grid[row][col] = ch;
                    }
                }
            }
        }

        let header = format!(
            "[window: {}x{}  panes: {}  session: {}]",
            win_cols,
            win_rows,
            panes.len(),
            session
        );
        println!("{}", header);
        println!("{}", separator_line(header.len()));

        // Trim trailing empty rows
        let mut last_nonempty = 0;
        for (i, row) in grid.iter().enumerate() {
            if row.iter().any(|c| !c.is_whitespace()) {
                last_nonempty = i;
            }
        }
        let display_rows = &grid[..=last_nonempty];
        let width = line_number_width(display_rows.len());
        for (i, row) in display_rows.iter().enumerate() {
            let line: String = row.iter().collect();
            println!("{:>width$}\u{2502} {}", i + 1, line, width = width);
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct WindowJsonSnapshot {
    session: String,
    window_size: Size,
    panes: Vec<PaneJsonEntry>,
}

#[derive(Serialize)]
struct PaneJsonEntry {
    pane_id: String,
    left: u16,
    top: u16,
    width: u16,
    height: u16,
    title: String,
    active: bool,
    size: Size,
    cursor: Cursor,
    lines: Vec<Line>,
}

fn output_window_json(
    session: &str,
    panes: &[PaneLayout],
    win_cols: u16,
    win_rows: u16,
) -> Result<(), String> {
    let mut entries = Vec::new();
    for p in panes {
        let ansi_content = capture_ansi(session, Some(&p.pane_id))?;
        let (cols, rows, cx, cy) = get_pane_info(session, Some(&p.pane_id))?;

        let all_lines: Vec<&str> = ansi_content.lines().collect();
        let lines = trim_trailing_empty(&all_lines);

        let json_lines: Vec<Line> = lines
            .iter()
            .enumerate()
            .map(|(i, raw_line)| {
                let (text, spans) = parse_ansi_line(raw_line);
                Line {
                    row: i + 1,
                    text,
                    spans,
                }
            })
            .collect();

        entries.push(PaneJsonEntry {
            pane_id: p.pane_id.clone(),
            left: p.left,
            top: p.top,
            width: p.width,
            height: p.height,
            title: p.title.clone(),
            active: p.active,
            size: Size { cols, rows },
            cursor: Cursor { row: cy, col: cx },
            lines: json_lines,
        });
    }

    let snapshot = WindowJsonSnapshot {
        session: session.to_string(),
        window_size: Size {
            cols: win_cols,
            rows: win_rows,
        },
        panes: entries,
    };

    let json = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| format!("JSON serialization failed: {}", e))?;
    println!("{}", json);
    Ok(())
}

pub fn scrollback_cmd(
    lines: Option<usize>,
    search: Option<&str>,
    session: &str,
) -> Result<(), String> {
    let target = target_str(session, None);

    // Build capture command
    let scroll_arg: String;
    let args = match lines {
        Some(n) => {
            scroll_arg = format!("-{}", n);
            vec!["capture-pane", "-t", &target, "-p", "-S", &scroll_arg]
        }
        None => {
            // Entire buffer
            vec!["capture-pane", "-t", &target, "-p", "-S", "-"]
        }
    };

    let output = Command::new("tmux")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run tmux capture-pane: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "tmux capture-pane failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let content = String::from_utf8_lossy(&output.stdout);

    match search {
        Some(pattern) => {
            let all_lines: Vec<&str> = content.lines().collect();
            let width = line_number_width(all_lines.len());
            let context = 3usize;
            let mut matched_ranges: Vec<(usize, usize)> = Vec::new();

            // Find matching line indices
            for (i, line) in all_lines.iter().enumerate() {
                if line.contains(pattern) {
                    let start = i.saturating_sub(context);
                    let end = (i + context + 1).min(all_lines.len());
                    matched_ranges.push((start, end));
                }
            }

            if matched_ranges.is_empty() {
                println!("No matches found for {:?}", pattern);
                return Ok(());
            }

            // Merge overlapping ranges
            let mut merged: Vec<(usize, usize)> = Vec::new();
            for (start, end) in &matched_ranges {
                if let Some(last) = merged.last_mut() {
                    if *start <= last.1 {
                        last.1 = last.1.max(*end);
                        continue;
                    }
                }
                merged.push((*start, *end));
            }

            // Print matched ranges with separators
            for (ri, (start, end)) in merged.iter().enumerate() {
                if ri > 0 {
                    println!("---");
                }
                for (i, line) in all_lines
                    .iter()
                    .enumerate()
                    .skip(*start)
                    .take(*end - *start)
                {
                    let marker = if line.contains(pattern) { ">" } else { " " };
                    println!(
                        "{}{:>width$}\u{2502} {}",
                        marker,
                        i + 1,
                        line,
                        width = width,
                    );
                }
            }
        }
        None => {
            let all_lines: Vec<&str> = content.lines().collect();
            let lines_trimmed = trim_trailing_empty(&all_lines);
            let width = line_number_width(lines_trimmed.len());
            for (i, line) in lines_trimmed.iter().enumerate() {
                println!("{:>width$}\u{2502} {}", i + 1, line, width = width);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_str() {
        assert_eq!(target_str("mysess", None), "mysess");
        assert_eq!(target_str("mysess", Some("mypane")), "mysess:mypane");
    }

    #[test]
    fn test_line_number_width() {
        assert_eq!(line_number_width(0), 1);
        assert_eq!(line_number_width(1), 2);
        assert_eq!(line_number_width(9), 2);
        assert_eq!(line_number_width(10), 2);
        assert_eq!(line_number_width(99), 2);
        assert_eq!(line_number_width(100), 3);
        assert_eq!(line_number_width(999), 3);
        assert_eq!(line_number_width(1000), 4);
    }

    #[test]
    fn test_trim_trailing_empty() {
        let lines = vec!["hello", "world", "", "  ", ""];
        let trimmed = trim_trailing_empty(&lines);
        assert_eq!(trimmed, &["hello", "world"]);
    }

    #[test]
    fn test_trim_trailing_empty_no_trailing() {
        let lines = vec!["hello", "world"];
        let trimmed = trim_trailing_empty(&lines);
        assert_eq!(trimmed, &["hello", "world"]);
    }

    #[test]
    fn test_trim_trailing_empty_all_empty() {
        let lines: Vec<&str> = vec!["", "", ""];
        let trimmed = trim_trailing_empty(&lines);
        assert!(trimmed.is_empty());
    }
}
