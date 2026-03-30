use serde::Serialize;
use std::process::Command;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

fn is_false(v: &bool) -> bool {
    !v
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Style {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub dim: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub underline: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub blink: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub reverse: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub strikethrough: bool,
}

impl Style {
    fn is_default(&self) -> bool {
        *self == Style::default()
    }

    /// Human-readable annotation string, e.g. `[fg:red bold underline]`.
    fn annotation(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ref fg) = self.fg {
            parts.push(format!("fg:{}", fg));
        }
        if let Some(ref bg) = self.bg {
            parts.push(format!("bg:{}", bg));
        }
        if self.bold {
            parts.push("bold".into());
        }
        if self.dim {
            parts.push("dim".into());
        }
        if self.italic {
            parts.push("italic".into());
        }
        if self.underline {
            parts.push("underline".into());
        }
        if self.blink {
            parts.push("blink".into());
        }
        if self.reverse {
            parts.push("reverse".into());
        }
        if self.strikethrough {
            parts.push("strikethrough".into());
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("[{}]", parts.join(" "))
        }
    }

    fn apply_sgr(&mut self, code: u32) {
        match code {
            0 => *self = Style::default(),
            1 => self.bold = true,
            2 => self.dim = true,
            3 => self.italic = true,
            4 => self.underline = true,
            5 | 6 => self.blink = true,
            7 => self.reverse = true,
            8 => {} // hidden — not tracked
            9 => self.strikethrough = true,
            21 => self.underline = true, // double underline, treat as underline
            22 => {
                self.bold = false;
                self.dim = false;
            }
            23 => self.italic = false,
            24 => self.underline = false,
            25 => self.blink = false,
            27 => self.reverse = false,
            28 => {} // reveal (undo hidden)
            29 => self.strikethrough = false,
            30..=37 => self.fg = Some(basic_color_name(code - 30)),
            38 => {} // handled by extended sequence caller
            39 => self.fg = None,
            40..=47 => self.bg = Some(basic_color_name(code - 40)),
            48 => {} // handled by extended sequence caller
            49 => self.bg = None,
            90..=97 => self.fg = Some(bright_color_name(code - 90)),
            100..=107 => self.bg = Some(bright_color_name(code - 100)),
            _ => {}
        }
    }
}

fn basic_color_name(idx: u32) -> String {
    match idx {
        0 => "black",
        1 => "red",
        2 => "green",
        3 => "yellow",
        4 => "blue",
        5 => "magenta",
        6 => "cyan",
        7 => "white",
        _ => "default",
    }
    .into()
}

fn bright_color_name(idx: u32) -> String {
    match idx {
        0 => "bright-black",
        1 => "bright-red",
        2 => "bright-green",
        3 => "bright-yellow",
        4 => "bright-blue",
        5 => "bright-magenta",
        6 => "bright-cyan",
        7 => "bright-white",
        _ => "default",
    }
    .into()
}

#[derive(Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    #[serde(flatten)]
    pub style: Style,
}

#[derive(Serialize)]
pub struct Line {
    pub row: usize,
    pub text: String,
    pub spans: Vec<Span>,
}

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
// ANSI parsing
// ---------------------------------------------------------------------------

/// Parse an ANSI-escaped string into a sequence of (plain_text, style) segments.
/// Each segment records the text content and the style that was active when it
/// was emitted.
pub fn parse_ansi(input: &str) -> Vec<(String, Style)> {
    let mut result: Vec<(String, Style)> = Vec::new();
    let mut style = Style::default();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut text_buf = String::new();

    while i < len {
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            // Flush accumulated text
            if !text_buf.is_empty() {
                result.push((std::mem::take(&mut text_buf), style.clone()));
            }
            // Parse CSI sequence: ESC [ params letter
            i += 2; // skip ESC [
            let seq_start = i;
            // Read until we find a final byte (0x40..=0x7E)
            while i < len && !(0x40..=0x7E).contains(&bytes[i]) {
                i += 1;
            }
            if i >= len {
                break;
            }
            let final_byte = bytes[i] as char;
            let params_str = std::str::from_utf8(&bytes[seq_start..i]).unwrap_or("");
            i += 1; // skip final byte

            if final_byte == 'm' {
                // SGR sequence
                apply_sgr_params(&mut style, params_str);
            }
            // Other CSI sequences are silently consumed (cursor movement, etc.)
        } else if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b']' {
            // OSC sequence: ESC ] ... ST
            // ST can be ESC \ or BEL (0x07)
            if !text_buf.is_empty() {
                result.push((std::mem::take(&mut text_buf), style.clone()));
            }
            i += 2;
            while i < len {
                if bytes[i] == 0x07 {
                    i += 1;
                    break;
                }
                if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'\\' {
                    i += 2;
                    break;
                }
                i += 1;
            }
        } else if bytes[i] == 0x1b {
            // Other escape sequences (e.g. ESC ( B, ESC ) 0) — skip two bytes
            if !text_buf.is_empty() {
                result.push((std::mem::take(&mut text_buf), style.clone()));
            }
            i += 1;
            // Skip the next character if present
            if i < len {
                i += 1;
            }
        } else {
            text_buf.push(bytes[i] as char);
            i += 1;
        }
    }

    // Flush remaining text
    if !text_buf.is_empty() {
        result.push((text_buf, style));
    }

    result
}

/// Apply a semicolon-separated SGR parameter string to a Style.
fn apply_sgr_params(style: &mut Style, params_str: &str) {
    if params_str.is_empty() {
        // ESC[m is equivalent to ESC[0m (reset)
        style.apply_sgr(0);
        return;
    }

    let parts: Vec<u32> = params_str
        .split(';')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();

    let mut j = 0;
    while j < parts.len() {
        let code = parts[j];
        match code {
            38 => {
                // Extended foreground color
                if j + 1 < parts.len() && parts[j + 1] == 5 {
                    // 256-color: 38;5;N
                    if j + 2 < parts.len() {
                        let n = parts[j + 2];
                        style.fg = Some(color_256_name(n));
                        j += 3;
                        continue;
                    }
                } else if j + 1 < parts.len() && parts[j + 1] == 2 {
                    // True color: 38;2;R;G;B
                    if j + 4 < parts.len() {
                        let r = parts[j + 2];
                        let g = parts[j + 3];
                        let b = parts[j + 4];
                        style.fg = Some(format!("rgb({},{},{})", r, g, b));
                        j += 5;
                        continue;
                    }
                }
                j += 1;
            }
            48 => {
                // Extended background color
                if j + 1 < parts.len() && parts[j + 1] == 5 {
                    // 256-color: 48;5;N
                    if j + 2 < parts.len() {
                        let n = parts[j + 2];
                        style.bg = Some(color_256_name(n));
                        j += 3;
                        continue;
                    }
                } else if j + 1 < parts.len() && parts[j + 1] == 2 {
                    // True color: 48;2;R;G;B
                    if j + 4 < parts.len() {
                        let r = parts[j + 2];
                        let g = parts[j + 3];
                        let b = parts[j + 4];
                        style.bg = Some(format!("rgb({},{},{})", r, g, b));
                        j += 5;
                        continue;
                    }
                }
                j += 1;
            }
            _ => {
                style.apply_sgr(code);
                j += 1;
            }
        }
    }
}

/// Convert a 256-color index to a human-readable name.
/// Indices 0-7 are the standard colors, 8-15 are bright, 16-231 are the
/// 6x6x6 color cube, and 232-255 are the grayscale ramp.
fn color_256_name(n: u32) -> String {
    match n {
        0 => "black".into(),
        1 => "red".into(),
        2 => "green".into(),
        3 => "yellow".into(),
        4 => "blue".into(),
        5 => "magenta".into(),
        6 => "cyan".into(),
        7 => "white".into(),
        8 => "bright-black".into(),
        9 => "bright-red".into(),
        10 => "bright-green".into(),
        11 => "bright-yellow".into(),
        12 => "bright-blue".into(),
        13 => "bright-magenta".into(),
        14 => "bright-cyan".into(),
        15 => "bright-white".into(),
        _ => format!("color({})", n),
    }
}

/// Parse a single line of ANSI-escaped text into (plain_text, spans).
pub fn parse_ansi_line(input: &str) -> (String, Vec<Span>) {
    let segments = parse_ansi(input);
    let mut plain = String::new();
    let mut spans: Vec<Span> = Vec::new();

    for (text, style) in &segments {
        if text.is_empty() {
            continue;
        }
        let start = plain.len();
        plain.push_str(text);
        let end = plain.len();

        // Merge with previous span if same style
        if let Some(last) = spans.last_mut() {
            if last.style == *style && last.end == start {
                last.end = end;
                continue;
            }
        }
        spans.push(Span {
            start,
            end,
            style: style.clone(),
        });
    }

    (plain, spans)
}

/// Determine the dominant style for a line by counting characters per style.
fn dominant_style(segments: &[(String, Style)]) -> Style {
    let mut best_style = Style::default();
    let mut best_count = 0usize;

    // Aggregate by style
    let mut style_counts: Vec<(Style, usize)> = Vec::new();
    for (text, style) in segments {
        let count = text.chars().filter(|c| !c.is_whitespace()).count();
        if count == 0 {
            continue;
        }
        if let Some(entry) = style_counts.iter_mut().find(|(s, _)| s == style) {
            entry.1 += count;
        } else {
            style_counts.push((style.clone(), count));
        }
    }

    for (style, count) in style_counts {
        if count > best_count {
            best_count = count;
            best_style = style;
        }
    }

    best_style
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
    let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return Err(format!("Unexpected window size output: {:?}", stdout.trim()));
    }
    let cols = parts[0].parse::<u16>().map_err(|e| format!("Bad window_width: {}", e))?;
    let rows = parts[1].parse::<u16>().map_err(|e| format!("Bad window_height: {}", e))?;
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
    let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
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

fn output_plain(
    content: &str,
    cols: u16,
    rows: u16,
    cx: u16,
    cy: u16,
    session: &str,
) {
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

fn output_color(
    ansi_content: &str,
    cols: u16,
    rows: u16,
    cx: u16,
    cy: u16,
    session: &str,
) {
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

fn output_ansi(
    ansi_content: &str,
    cols: u16,
    rows: u16,
    cx: u16,
    cy: u16,
    session: &str,
) {
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

fn output_diff(
    content: &str,
    session: &str,
    cols: u16,
    rows: u16,
    cx: u16,
    cy: u16,
) {
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
                println!(
                    "-{:>width$}\u{2502} {}",
                    i + 1,
                    prev,
                    width = width,
                );
            }
            println!(
                "+{:>width$}\u{2502} {}",
                i + 1,
                cur,
                width = width,
            );
        } else {
            println!(
                " {:>width$}\u{2502} {}",
                i + 1,
                cur,
                width = width,
            );
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
            win_cols, win_rows, panes.len(), session
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
            win_cols, win_rows, panes.len(), session
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
        window_size: Size { cols: win_cols, rows: win_rows },
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
                for i in *start..*end {
                    let marker = if all_lines[i].contains(pattern) {
                        ">"
                    } else {
                        " "
                    };
                    println!(
                        "{}{:>width$}\u{2502} {}",
                        marker,
                        i + 1,
                        all_lines[i],
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
    fn test_parse_ansi_empty() {
        let result = parse_ansi("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_ansi_plain_text() {
        let result = parse_ansi("hello world");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello world");
        assert!(result[0].1.is_default());
    }

    #[test]
    fn test_parse_ansi_bold() {
        let result = parse_ansi("\x1b[1mhello\x1b[0m world");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "hello");
        assert!(result[0].1.bold);
        assert_eq!(result[1].0, " world");
        assert!(result[1].1.is_default());
    }

    #[test]
    fn test_parse_ansi_fg_color() {
        let result = parse_ansi("\x1b[31mred text\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "red text");
        assert_eq!(result[0].1.fg, Some("red".into()));
    }

    #[test]
    fn test_parse_ansi_combined() {
        // bold + green foreground
        let result = parse_ansi("\x1b[1;32mGreen Bold\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Green Bold");
        assert!(result[0].1.bold);
        assert_eq!(result[0].1.fg, Some("green".into()));
    }

    #[test]
    fn test_parse_ansi_256_color() {
        let result = parse_ansi("\x1b[38;5;196mhello\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.fg, Some("color(196)".into()));
    }

    #[test]
    fn test_parse_ansi_truecolor() {
        let result = parse_ansi("\x1b[38;2;255;128;0mhello\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.fg, Some("rgb(255,128,0)".into()));
    }

    #[test]
    fn test_parse_ansi_bg_color() {
        let result = parse_ansi("\x1b[44mblue bg\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.bg, Some("blue".into()));
    }

    #[test]
    fn test_parse_ansi_bright_colors() {
        let result = parse_ansi("\x1b[91mbright red\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.fg, Some("bright-red".into()));
    }

    #[test]
    fn test_parse_ansi_underline_and_italic() {
        let result = parse_ansi("\x1b[3;4mfancy\x1b[0m");
        assert_eq!(result.len(), 1);
        assert!(result[0].1.italic);
        assert!(result[0].1.underline);
    }

    #[test]
    fn test_parse_ansi_reverse() {
        let result = parse_ansi("\x1b[7mreversed\x1b[0m");
        assert_eq!(result.len(), 1);
        assert!(result[0].1.reverse);
    }

    #[test]
    fn test_parse_ansi_strikethrough() {
        let result = parse_ansi("\x1b[9mstruck\x1b[0m");
        assert_eq!(result.len(), 1);
        assert!(result[0].1.strikethrough);
    }

    #[test]
    fn test_parse_ansi_reset_mid_stream() {
        let result = parse_ansi("\x1b[1mbold\x1b[0mnormal");
        assert_eq!(result.len(), 2);
        assert!(result[0].1.bold);
        assert!(result[1].1.is_default());
    }

    #[test]
    fn test_parse_ansi_default_fg() {
        let result = parse_ansi("\x1b[31mred\x1b[39mdefault");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1.fg, Some("red".into()));
        assert_eq!(result[1].1.fg, None);
    }

    #[test]
    fn test_parse_ansi_line_basic() {
        let (text, spans) = parse_ansi_line("\x1b[31mhello\x1b[0m world");
        assert_eq!(text, "hello world");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 5);
        assert_eq!(spans[0].style.fg, Some("red".into()));
        assert_eq!(spans[1].start, 5);
        assert_eq!(spans[1].end, 11);
        assert!(spans[1].style.is_default());
    }

    #[test]
    fn test_parse_ansi_bare_esc_m() {
        // ESC[m should reset (same as ESC[0m)
        let result = parse_ansi("\x1b[1mbold\x1b[mnormal");
        assert_eq!(result.len(), 2);
        assert!(result[0].1.bold);
        assert!(result[1].1.is_default());
    }

    #[test]
    fn test_parse_ansi_256_bg() {
        let result = parse_ansi("\x1b[48;5;232mtext\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.bg, Some("color(232)".into()));
    }

    #[test]
    fn test_parse_ansi_truecolor_bg() {
        let result = parse_ansi("\x1b[48;2;10;20;30mtext\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.bg, Some("rgb(10,20,30)".into()));
    }

    #[test]
    fn test_parse_ansi_fg_and_bg_combined() {
        let result = parse_ansi("\x1b[33;44mYellow on Blue\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.fg, Some("yellow".into()));
        assert_eq!(result[0].1.bg, Some("blue".into()));
    }

    #[test]
    fn test_parse_ansi_multiple_resets() {
        let result = parse_ansi("\x1b[1;4;31mstuff\x1b[22m\x1b[24mplain");
        // After processing: bold=true, underline=true, fg=red -> then 22 clears bold/dim -> 24 clears underline
        assert_eq!(result.len(), 2);
        assert!(result[0].1.bold);
        assert!(result[0].1.underline);
        assert_eq!(result[0].1.fg, Some("red".into()));
        // Second segment: bold and underline cleared, fg still red
        assert!(!result[1].1.bold);
        assert!(!result[1].1.underline);
        assert_eq!(result[1].1.fg, Some("red".into()));
    }

    #[test]
    fn test_style_annotation() {
        let mut s = Style::default();
        assert_eq!(s.annotation(), "");

        s.fg = Some("red".into());
        s.bold = true;
        assert_eq!(s.annotation(), "[fg:red bold]");

        s.bg = Some("blue".into());
        s.underline = true;
        assert_eq!(s.annotation(), "[fg:red bg:blue bold underline]");
    }

    #[test]
    fn test_dominant_style() {
        let segments = vec![
            ("hello".into(), Style { fg: Some("red".into()), bold: true, ..Style::default() }),
            ("  ".into(), Style::default()),
            ("wo".into(), Style { fg: Some("blue".into()), ..Style::default() }),
        ];
        let dom = dominant_style(&segments);
        // "hello" has 5 non-whitespace chars with red+bold, "wo" has 2 with blue
        assert_eq!(dom.fg, Some("red".into()));
        assert!(dom.bold);
    }

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

    #[test]
    fn test_parse_ansi_osc_sequence() {
        // OSC terminated by BEL
        let result = parse_ansi("\x1b]0;title\x07hello");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello");
    }

    #[test]
    fn test_parse_ansi_osc_st_terminated() {
        // OSC terminated by ST (ESC \)
        let result = parse_ansi("\x1b]0;title\x1b\\hello");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello");
    }

    #[test]
    fn test_parse_ansi_non_sgr_csi() {
        // Cursor movement sequences should be stripped
        let result = parse_ansi("\x1b[2Jhello\x1b[Hworld");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "hello");
        assert_eq!(result[1].0, "world");
    }

    #[test]
    fn test_parse_ansi_bright_bg() {
        let result = parse_ansi("\x1b[100mtext\x1b[0m");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.bg, Some("bright-black".into()));
    }

    #[test]
    fn test_parse_ansi_dim() {
        let result = parse_ansi("\x1b[2mdim\x1b[0m");
        assert_eq!(result.len(), 1);
        assert!(result[0].1.dim);
    }

    #[test]
    fn test_parse_ansi_blink() {
        let result = parse_ansi("\x1b[5mblink\x1b[0m");
        assert_eq!(result.len(), 1);
        assert!(result[0].1.blink);
    }

    #[test]
    fn test_style_serialize_skips_false() {
        let s = Style {
            fg: Some("red".into()),
            bold: true,
            ..Style::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"fg\":\"red\""));
        assert!(json.contains("\"bold\":true"));
        assert!(!json.contains("\"dim\""));
        assert!(!json.contains("\"italic\""));
        assert!(!json.contains("\"bg\""));
    }

    #[test]
    fn test_parse_ansi_line_merged_spans() {
        // Two adjacent segments with same style should merge
        let (text, spans) = parse_ansi_line("hello world");
        assert_eq!(text, "hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 11);
    }

    #[test]
    fn test_color_256_standard() {
        assert_eq!(color_256_name(0), "black");
        assert_eq!(color_256_name(7), "white");
        assert_eq!(color_256_name(8), "bright-black");
        assert_eq!(color_256_name(15), "bright-white");
        assert_eq!(color_256_name(196), "color(196)");
        assert_eq!(color_256_name(232), "color(232)");
    }
}
