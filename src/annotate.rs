use std::fs;
use crate::snapshot;

/// Render a terminal screenshot as PNG or HTML.
pub fn screenshot(
    path: Option<&str>,
    annotate: bool,
    html: bool,
    theme: &str,
    session: &str,
) -> Result<(), String> {
    // Capture the ANSI content
    let ansi_content = snapshot::capture_ansi(session, None)
        .map_err(|e| format!("Failed to capture pane: {}", e))?;

    let (cols, rows, cursor_x, cursor_y) = snapshot::get_pane_info(session, None)
        .map_err(|e| format!("Failed to get pane info: {}", e))?;

    if html {
        let output_path = path.unwrap_or("screenshot.html");
        let html_content = render_html(&ansi_content, cols, rows, cursor_x, cursor_y, annotate, theme);
        fs::write(output_path, &html_content)
            .map_err(|e| format!("Failed to write HTML: {}", e))?;
        println!("Screenshot saved to {}", output_path);
    } else {
        let output_path = path.unwrap_or("screenshot.png");
        render_png(&ansi_content, cols, rows, cursor_x, cursor_y, annotate, theme, output_path)?;
        println!("Screenshot saved to {}", output_path);
    }

    Ok(())
}

/// Render terminal content as HTML with inline CSS.
fn render_html(
    ansi_content: &str,
    cols: u16,
    rows: u16,
    cursor_x: u16,
    cursor_y: u16,
    annotate: bool,
    theme: &str,
) -> String {
    let (bg_color, fg_color) = if theme == "light" {
        ("#ffffff", "#000000")
    } else {
        ("#1e1e1e", "#d4d4d4")
    };

    let color_map = AnsiColorMap::new(theme);

    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str(&format!(
        "<title>agent-terminal screenshot ({}x{})</title>\n",
        cols, rows
    ));
    html.push_str("<style>\n");
    html.push_str(&format!(
        "body {{ margin: 0; padding: 20px; background: {}; }}\n",
        bg_color
    ));
    html.push_str(".terminal {\n");
    html.push_str(&format!("  background: {};\n", bg_color));
    html.push_str(&format!("  color: {};\n", fg_color));
    html.push_str("  font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;\n");
    html.push_str("  font-size: 14px;\n");
    html.push_str("  line-height: 1.4;\n");
    html.push_str("  padding: 16px;\n");
    html.push_str("  border-radius: 8px;\n");
    html.push_str("  white-space: pre;\n");
    html.push_str("  overflow: auto;\n");
    html.push_str("  border: 1px solid #333;\n");
    html.push_str("}\n");
    html.push_str(".title-bar {\n");
    html.push_str("  background: #333;\n");
    html.push_str("  color: #aaa;\n");
    html.push_str("  padding: 6px 16px;\n");
    html.push_str("  border-radius: 8px 8px 0 0;\n");
    html.push_str("  font-family: sans-serif;\n");
    html.push_str("  font-size: 12px;\n");
    html.push_str("}\n");
    if annotate {
        html.push_str(".row-num { color: #666; user-select: none; }\n");
        html.push_str(".col-ruler { color: #444; user-select: none; display: block; border-bottom: 1px solid #333; margin-bottom: 4px; }\n");
    }
    html.push_str(".bold { font-weight: bold; }\n");
    html.push_str(".dim { opacity: 0.5; }\n");
    html.push_str(".italic { font-style: italic; }\n");
    html.push_str(".underline { text-decoration: underline; }\n");
    html.push_str(".strikethrough { text-decoration: line-through; }\n");
    html.push_str(".cursor { background: #ffffff40; outline: 1px solid #fff; }\n");
    html.push_str("</style>\n</head>\n<body>\n");

    html.push_str(&format!(
        "<div class=\"title-bar\">agent-terminal — {}x{} — cursor: {},{}</div>\n",
        cols, rows, cursor_y, cursor_x
    ));
    html.push_str("<div class=\"terminal\">");

    // Column ruler if annotating
    if annotate {
        html.push_str("<span class=\"col-ruler\">");
        html.push_str("    ");
        let mut ruler = String::new();
        for i in 1..=cols {
            if i % 10 == 0 {
                ruler.push_str(&format!("{}", (i / 10) % 10));
            } else if i % 5 == 0 {
                ruler.push('+');
            } else {
                ruler.push('·');
            }
        }
        html.push_str(&ruler);
        html.push_str("</span>\n");
    }

    let lines: Vec<&str> = ansi_content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if annotate {
            html.push_str(&format!(
                "<span class=\"row-num\">{:>3}│</span>",
                i + 1
            ));
        }

        // Parse ANSI in this line and convert to HTML spans
        let html_line = ansi_line_to_html(line, &color_map, i, cursor_x as usize, cursor_y as usize);
        html.push_str(&html_line);
        html.push('\n');
    }

    html.push_str("</div>\n</body>\n</html>");
    html
}

/// Convert a single ANSI line to HTML spans.
fn ansi_line_to_html(line: &str, color_map: &AnsiColorMap, row: usize, cursor_x: usize, cursor_y: usize) -> String {
    let mut result = String::new();
    let mut chars = line.chars().peekable();
    let mut current_style = StyleState::default();
    let mut col = 0;

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Parse escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                let mut params = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == ';' {
                        params.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(&cmd) = chars.peek() {
                    chars.next();
                    if cmd == 'm' {
                        // SGR sequence
                        if !current_style.is_default() {
                            result.push_str("</span>");
                        }
                        current_style.apply_sgr(&params);
                        if !current_style.is_default() {
                            result.push_str(&current_style.to_html_open(color_map));
                        }
                    }
                }
            }
        } else {
            let is_cursor = row == cursor_y as usize && col == cursor_x as usize;
            if is_cursor {
                result.push_str("<span class=\"cursor\">");
            }
            match ch {
                '<' => result.push_str("&lt;"),
                '>' => result.push_str("&gt;"),
                '&' => result.push_str("&amp;"),
                '"' => result.push_str("&quot;"),
                _ => result.push(ch),
            }
            if is_cursor {
                result.push_str("</span>");
            }
            col += 1;
        }
    }

    if !current_style.is_default() {
        result.push_str("</span>");
    }

    result
}

/// Render terminal content as PNG using the image crate.
fn render_png(
    ansi_content: &str,
    cols: u16,
    _rows: u16,
    cursor_x: u16,
    cursor_y: u16,
    annotate: bool,
    theme: &str,
    output_path: &str,
) -> Result<(), String> {
    let cell_width: u32 = 8;
    let cell_height: u32 = 16;
    let padding: u32 = 16;
    let gutter: u32 = if annotate { 40 } else { 0 };
    let title_bar_height: u32 = 28;

    let lines: Vec<&str> = ansi_content.lines().collect();
    let num_lines = lines.len().max(1) as u32;

    let img_width = padding * 2 + gutter + (cols as u32 * cell_width);
    let img_height = padding * 2 + title_bar_height + (num_lines * cell_height);

    let (bg_r, bg_g, bg_b) = if theme == "light" {
        (255u8, 255u8, 255u8)
    } else {
        (30u8, 30u8, 30u8)
    };

    let (fg_r, fg_g, fg_b) = if theme == "light" {
        (0u8, 0u8, 0u8)
    } else {
        (212u8, 212u8, 212u8)
    };

    let mut imgbuf = image::RgbaImage::from_pixel(
        img_width,
        img_height,
        image::Rgba([bg_r, bg_g, bg_b, 255]),
    );

    // Draw title bar background
    for y in 0..title_bar_height {
        for x in 0..img_width {
            imgbuf.put_pixel(x, y, image::Rgba([51, 51, 51, 255]));
        }
    }

    // Render text character by character using basic bitmap approach
    // Since we can't easily embed fonts without complex setup, we'll render
    // each character as a simple bitmap pattern
    let y_offset = title_bar_height + padding;
    let x_offset = padding + gutter;

    for (line_idx, line) in lines.iter().enumerate() {
        let y_pos = y_offset + (line_idx as u32 * cell_height);

        // Draw row number if annotating
        if annotate {
            let num_str = format!("{:>3}│", line_idx + 1);
            draw_simple_text(
                &mut imgbuf,
                &num_str,
                padding,
                y_pos,
                cell_width,
                cell_height,
                [100, 100, 100, 255],
            );
        }

        // Strip ANSI and render plain text with coloring
        let plain = strip_ansi(line);
        let color = [fg_r, fg_g, fg_b, 255];

        draw_simple_text(
            &mut imgbuf,
            &plain,
            x_offset,
            y_pos,
            cell_width,
            cell_height,
            color,
        );

        // Draw cursor
        if line_idx == cursor_y as usize {
            let cx = x_offset + (cursor_x as u32 * cell_width);
            let cy = y_pos;
            for dy in 0..cell_height {
                for dx in 0..cell_width {
                    if cx + dx < img_width && cy + dy < img_height {
                        let pixel = imgbuf.get_pixel(cx + dx, cy + dy);
                        // Invert colors for cursor
                        imgbuf.put_pixel(
                            cx + dx,
                            cy + dy,
                            image::Rgba([255 - pixel[0], 255 - pixel[1], 255 - pixel[2], 255]),
                        );
                    }
                }
            }
        }
    }

    imgbuf
        .save(output_path)
        .map_err(|e| format!("Failed to save PNG: {}", e))?;

    Ok(())
}

/// Draw text as simple block characters (bitmap approximation).
/// This is a simplified renderer - for production, you'd use ab_glyph with an embedded font.
fn draw_simple_text(
    img: &mut image::RgbaImage,
    text: &str,
    x: u32,
    y: u32,
    cell_w: u32,
    _cell_h: u32,
    color: [u8; 4],
) {
    for (i, ch) in text.chars().enumerate() {
        if ch == ' ' || ch == '\t' {
            continue;
        }
        let cx = x + (i as u32 * cell_w);
        // Draw a simple representation of the character
        // For a real implementation, we'd use ab_glyph to rasterize glyphs
        // Here we draw a simple pattern that represents the character
        let pattern = get_char_pattern(ch);
        for (dy, row) in pattern.iter().enumerate() {
            for (dx, &pixel) in row.iter().enumerate() {
                if pixel > 0 {
                    let px = cx + dx as u32;
                    let py = y + dy as u32 + 2; // offset from top of cell
                    if px < img.width() && py < img.height() {
                        let alpha = (pixel as u16 * color[3] as u16 / 255) as u8;
                        img.put_pixel(px, py, image::Rgba([color[0], color[1], color[2], alpha]));
                    }
                }
            }
        }
    }
}

/// Get a simple 6x10 bitmap pattern for a character.
/// Returns a 10-row by 6-col grid of alpha values (0 or 255).
fn get_char_pattern(ch: char) -> Vec<Vec<u8>> {
    // Simple 6x10 bitmap font for basic ASCII characters
    // This is a minimal implementation - a real one would embed a proper font
    let empty = vec![vec![0u8; 6]; 10];

    // For characters we don't have patterns for, draw a filled rectangle
    if !ch.is_ascii_graphic() {
        return empty;
    }

    // Draw a simple filled block to represent any character
    // In production, we'd use ab_glyph with an embedded monospace font
    let mut pattern = vec![vec![0u8; 6]; 10];

    // Simple block representation - sufficient for screenshot thumbnails
    // Characters are represented as small filled areas
    match ch {
        '│' | '|' => {
            for row in &mut pattern {
                row[3] = 255;
            }
        }
        '─' | '-' => {
            pattern[5] = vec![255; 6];
        }
        _ => {
            // Generic character: draw a small filled rectangle
            for row in pattern.iter_mut().take(9).skip(1) {
                for col in row.iter_mut().take(5).skip(1) {
                    *col = 200;
                }
            }
        }
    }

    pattern
}

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip escape sequence
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphabetic() {
                        chars.next();
                        break;
                    }
                    chars.next();
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

// --- Style tracking for HTML rendering ---

#[derive(Default, Clone)]
struct StyleState {
    fg: Option<(u8, u8, u8)>,
    bg: Option<(u8, u8, u8)>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    reverse: bool,
    strikethrough: bool,
}

impl StyleState {
    fn is_default(&self) -> bool {
        self.fg.is_none()
            && self.bg.is_none()
            && !self.bold
            && !self.dim
            && !self.italic
            && !self.underline
            && !self.reverse
            && !self.strikethrough
    }

    fn apply_sgr(&mut self, params: &str) {
        if params.is_empty() {
            *self = StyleState::default();
            return;
        }

        let nums: Vec<u32> = params
            .split(';')
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut i = 0;
        while i < nums.len() {
            match nums[i] {
                0 => *self = StyleState::default(),
                1 => self.bold = true,
                2 => self.dim = true,
                3 => self.italic = true,
                4 => self.underline = true,
                7 => self.reverse = true,
                9 => self.strikethrough = true,
                22 => { self.bold = false; self.dim = false; }
                23 => self.italic = false,
                24 => self.underline = false,
                27 => self.reverse = false,
                29 => self.strikethrough = false,
                30..=37 => {
                    self.fg = Some(ansi_basic_color(nums[i] - 30));
                }
                38 => {
                    if i + 1 < nums.len() && nums[i + 1] == 5 && i + 2 < nums.len() {
                        self.fg = Some(ansi_256_color(nums[i + 2] as u8));
                        i += 2;
                    } else if i + 1 < nums.len() && nums[i + 1] == 2 && i + 4 < nums.len() {
                        self.fg = Some((nums[i + 2] as u8, nums[i + 3] as u8, nums[i + 4] as u8));
                        i += 4;
                    }
                }
                39 => self.fg = None,
                40..=47 => {
                    self.bg = Some(ansi_basic_color(nums[i] - 40));
                }
                48 => {
                    if i + 1 < nums.len() && nums[i + 1] == 5 && i + 2 < nums.len() {
                        self.bg = Some(ansi_256_color(nums[i + 2] as u8));
                        i += 2;
                    } else if i + 1 < nums.len() && nums[i + 1] == 2 && i + 4 < nums.len() {
                        self.bg = Some((nums[i + 2] as u8, nums[i + 3] as u8, nums[i + 4] as u8));
                        i += 4;
                    }
                }
                49 => self.bg = None,
                90..=97 => {
                    self.fg = Some(ansi_bright_color(nums[i] - 90));
                }
                100..=107 => {
                    self.bg = Some(ansi_bright_color(nums[i] - 100));
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn to_html_open(&self, _color_map: &AnsiColorMap) -> String {
        let mut style_parts = Vec::new();
        let mut classes = Vec::new();

        if let Some((r, g, b)) = self.fg {
            style_parts.push(format!("color:rgb({},{},{})", r, g, b));
        }
        if let Some((r, g, b)) = self.bg {
            style_parts.push(format!("background:rgb({},{},{})", r, g, b));
        }
        if self.bold {
            classes.push("bold");
        }
        if self.dim {
            classes.push("dim");
        }
        if self.italic {
            classes.push("italic");
        }
        if self.underline {
            classes.push("underline");
        }
        if self.strikethrough {
            classes.push("strikethrough");
        }

        let class_attr = if classes.is_empty() {
            String::new()
        } else {
            format!(" class=\"{}\"", classes.join(" "))
        };

        let style_attr = if style_parts.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", style_parts.join(";"))
        };

        format!("<span{}{}>", class_attr, style_attr)
    }
}

struct AnsiColorMap {
    _theme: String,
}

impl AnsiColorMap {
    fn new(theme: &str) -> Self {
        AnsiColorMap {
            _theme: theme.to_string(),
        }
    }
}

fn ansi_basic_color(idx: u32) -> (u8, u8, u8) {
    match idx {
        0 => (0, 0, 0),       // black
        1 => (205, 49, 49),    // red
        2 => (13, 188, 121),   // green
        3 => (229, 229, 16),   // yellow
        4 => (36, 114, 200),   // blue
        5 => (188, 63, 188),   // magenta
        6 => (17, 168, 205),   // cyan
        7 => (229, 229, 229),  // white
        _ => (229, 229, 229),
    }
}

fn ansi_bright_color(idx: u32) -> (u8, u8, u8) {
    match idx {
        0 => (102, 102, 102),  // bright black
        1 => (241, 76, 76),    // bright red
        2 => (35, 209, 139),   // bright green
        3 => (245, 245, 67),   // bright yellow
        4 => (59, 142, 234),   // bright blue
        5 => (214, 112, 214),  // bright magenta
        6 => (41, 184, 219),   // bright cyan
        7 => (255, 255, 255),  // bright white
        _ => (255, 255, 255),
    }
}

fn ansi_256_color(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=7 => ansi_basic_color(idx as u32),
        8..=15 => ansi_bright_color((idx - 8) as u32),
        16..=231 => {
            // 6x6x6 color cube
            let idx = idx - 16;
            let r = (idx / 36) % 6;
            let g = (idx / 6) % 6;
            let b = idx % 6;
            let to_val = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            (to_val(r), to_val(g), to_val(b))
        }
        232..=255 => {
            // Grayscale ramp
            let v = 8 + 10 * (idx - 232);
            (v, v, v)
        }
    }
}
