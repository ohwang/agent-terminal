use serde::Serialize;

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
    pub fn is_default(&self) -> bool {
        *self == Style::default()
    }

    /// Human-readable annotation string, e.g. `[fg:red bold underline]`.
    pub fn annotation(&self) -> String {
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
pub fn dominant_style(segments: &[(String, Style)]) -> Style {
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
// Style matching (for assertions and find)
// ---------------------------------------------------------------------------

/// Parse a style specification string like "fg:red,bold,underline" into a Style.
pub fn parse_style_spec(spec: &str) -> Style {
    let mut style = Style::default();
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
                "blink" => style.blink = true,
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
pub fn style_matches(actual: &Style, required: &Style) -> bool {
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
    if required.blink && !actual.blink {
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
        assert_eq!(result.len(), 2);
        assert!(result[0].1.bold);
        assert!(result[0].1.underline);
        assert_eq!(result[0].1.fg, Some("red".into()));
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
            (
                "hello".into(),
                Style {
                    fg: Some("red".into()),
                    bold: true,
                    ..Style::default()
                },
            ),
            ("  ".into(), Style::default()),
            (
                "wo".into(),
                Style {
                    fg: Some("blue".into()),
                    ..Style::default()
                },
            ),
        ];
        let dom = dominant_style(&segments);
        assert_eq!(dom.fg, Some("red".into()));
        assert!(dom.bold);
    }

    #[test]
    fn test_parse_ansi_line_merged_spans() {
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

    #[test]
    fn test_parse_ansi_osc_sequence() {
        let result = parse_ansi("\x1b]0;title\x07hello");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello");
    }

    #[test]
    fn test_parse_ansi_osc_st_terminated() {
        let result = parse_ansi("\x1b]0;title\x1b\\hello");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello");
    }

    #[test]
    fn test_parse_ansi_non_sgr_csi() {
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
    fn test_parse_style_spec() {
        let style = parse_style_spec("fg:red,bold,underline");
        assert_eq!(style.fg, Some("red".to_string()));
        assert!(style.bold);
        assert!(style.underline);
        assert!(!style.italic);
    }

    #[test]
    fn test_parse_style_spec_bg() {
        let style = parse_style_spec("bg:blue,italic");
        assert_eq!(style.bg, Some("blue".to_string()));
        assert!(style.italic);
    }

    #[test]
    fn test_style_matches_subset() {
        let actual = Style {
            fg: Some("red".into()),
            bold: true,
            underline: true,
            ..Style::default()
        };
        let required = parse_style_spec("fg:red,bold");
        assert!(style_matches(&actual, &required));
    }

    #[test]
    fn test_style_matches_mismatch() {
        let actual = Style {
            fg: Some("green".into()),
            ..Style::default()
        };
        let required = parse_style_spec("fg:red");
        assert!(!style_matches(&actual, &required));
    }

    #[test]
    fn test_style_matches_empty_spec() {
        let actual = Style {
            fg: Some("red".into()),
            bold: true,
            ..Style::default()
        };
        let required = Style::default();
        assert!(style_matches(&actual, &required));
    }
}
