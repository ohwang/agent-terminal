use std::fs;
use ab_glyph::{Font as AbGlyphFont, FontVec, PxScale, ScaleFont, point};
use crate::snapshot;

fn default_screenshot_path(session: &str, ext: &str) -> String {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    format!("{}-{}.{}", session, ts, ext)
}

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
        let default_path = default_screenshot_path(session, "html");
        let output_path = path.unwrap_or(&default_path);
        let html_content = render_html(&ansi_content, cols, rows, cursor_x, cursor_y, annotate, theme);
        fs::write(output_path, &html_content)
            .map_err(|e| format!("Failed to write HTML: {}", e))?;
        println!("Screenshot saved to {}", output_path);
    } else {
        let default_path = default_screenshot_path(session, "png");
        let output_path = path.unwrap_or(&default_path);
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

/// A single visible character with its resolved colors.
struct ColoredCell {
    ch: char,
    fg: (u8, u8, u8),
    bg: Option<(u8, u8, u8)>,
}

/// Parse an ANSI line into colored cells, resolving all SGR sequences.
fn parse_ansi_line_to_cells(line: &str, default_fg: (u8, u8, u8), style: &mut StyleState) -> Vec<ColoredCell> {
    let mut cells = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
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
                        style.apply_sgr(&params);
                    }
                }
            }
        } else {
            let (fg, bg) = if style.reverse {
                (
                    style.bg.unwrap_or(default_fg),
                    Some(style.fg.unwrap_or(default_fg)),
                )
            } else {
                (style.fg.unwrap_or(default_fg), style.bg)
            };
            cells.push(ColoredCell { ch, fg, bg });
        }
    }

    cells
}

/// Load a monospace font from the system.
fn load_font() -> Result<FontVec, String> {
    let candidates: &[(&str, Option<u32>)] = &[
        // macOS
        ("/System/Library/Fonts/Menlo.ttc", Some(0)),
        ("/System/Library/Fonts/Monaco.ttf", None),
        ("/System/Library/Fonts/Supplemental/Courier New.ttf", None),
        // Linux
        ("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", None),
        ("/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf", None),
        ("/usr/share/fonts/TTF/DejaVuSansMono.ttf", None),
        ("/usr/share/fonts/truetype/freefont/FreeMono.ttf", None),
        // Windows
        ("C:\\Windows\\Fonts\\consola.ttf", None),
        ("C:\\Windows\\Fonts\\cour.ttf", None),
    ];

    for (path, index) in candidates {
        if let Ok(data) = fs::read(path) {
            let result = if let Some(idx) = index {
                FontVec::try_from_vec_and_index(data, *idx)
            } else {
                FontVec::try_from_vec(data)
            };
            match result {
                Ok(font) => return Ok(font),
                Err(_) => continue,
            }
        }
    }

    Err("No monospace font found. Searched: Menlo (macOS), DejaVu Sans Mono (Linux), Consolas (Windows)".into())
}

/// Draw a single glyph onto the image with alpha blending.
fn draw_glyph(
    img: &mut image::RgbaImage,
    font: &FontVec,
    ch: char,
    scale: PxScale,
    x: f32,
    baseline_y: f32,
    color: (u8, u8, u8),
) {
    let glyph_id = font.glyph_id(ch);
    let glyph = glyph_id.with_scale_and_position(scale, point(x, baseline_y));

    if let Some(outlined) = font.outline_glyph(glyph) {
        let (r, g, b) = color;
        let bounds = outlined.px_bounds();
        let bx = bounds.min.x as u32;
        let by = bounds.min.y as u32;
        let img_w = img.width();
        let img_h = img.height();
        outlined.draw(|rx, ry, coverage| {
            let px = rx + bx;
            let py = ry + by;
            if px < img_w && py < img_h {
                let alpha = (coverage * 255.0) as u16;
                if alpha == 0 {
                    return;
                }
                let bg = img.get_pixel(px, py);
                let inv = 255 - alpha;
                let blended_r = ((r as u16 * alpha + bg[0] as u16 * inv) / 255) as u8;
                let blended_g = ((g as u16 * alpha + bg[1] as u16 * inv) / 255) as u8;
                let blended_b = ((b as u16 * alpha + bg[2] as u16 * inv) / 255) as u8;
                img.put_pixel(px, py, image::Rgba([blended_r, blended_g, blended_b, 255]));
            }
        });
    }
}

/// Render terminal content as PNG using ab_glyph for real font rendering.
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
    let font = load_font()?;
    let font_size = 26.0_f32;
    let scale = PxScale::from(font_size);
    let scaled = font.as_scaled(scale);

    let cell_width = scaled.h_advance(font.glyph_id('M')).ceil() as u32;
    let ascent = scaled.ascent();
    let descent = -scaled.descent(); // descent is negative
    let cell_height = (ascent + descent).ceil() as u32;

    let padding: u32 = 16;
    let gutter: u32 = if annotate { cell_width * 4 } else { 0 };
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

    let default_fg = if theme == "light" {
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

    let y_offset = title_bar_height + padding;
    let x_offset = padding + gutter;
    let gutter_color = (100u8, 100u8, 100u8);

    // Style state persists across lines (ANSI colors can span lines)
    let mut style = StyleState::default();

    for (line_idx, line) in lines.iter().enumerate() {
        let y_pos = y_offset + (line_idx as u32 * cell_height);
        let baseline_y = y_pos as f32 + ascent;

        // Draw row numbers in the gutter
        if annotate {
            let num_str = format!("{:>3}|", line_idx + 1);
            for (i, ch) in num_str.chars().enumerate() {
                let gx = padding + (i as u32 * cell_width);
                draw_glyph(&mut imgbuf, &font, ch, scale, gx as f32, baseline_y, gutter_color);
            }
        }

        // Parse ANSI and render each cell with proper color
        let cells = parse_ansi_line_to_cells(line, default_fg, &mut style);

        for (col_idx, cell) in cells.iter().enumerate() {
            let cx = x_offset + (col_idx as u32 * cell_width);
            let cy = y_pos;

            // Draw cell background if set
            if let Some((br, bg, bb)) = cell.bg {
                for dy in 0..cell_height {
                    for dx in 0..cell_width {
                        if cx + dx < img_width && cy + dy < img_height {
                            imgbuf.put_pixel(cx + dx, cy + dy, image::Rgba([br, bg, bb, 255]));
                        }
                    }
                }
            }

            // Draw the character glyph
            if cell.ch != ' ' {
                draw_glyph(&mut imgbuf, &font, cell.ch, scale, cx as f32, baseline_y, cell.fg);
            }
        }

        // Draw cursor (invert colors at cursor position)
        if line_idx == cursor_y as usize {
            let cx = x_offset + (cursor_x as u32 * cell_width);
            let cy = y_pos;
            for dy in 0..cell_height {
                for dx in 0..cell_width {
                    if cx + dx < img_width && cy + dy < img_height {
                        let pixel = imgbuf.get_pixel(cx + dx, cy + dy);
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
