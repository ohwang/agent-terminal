use crate::snapshot;
use ab_glyph::{point, Font as AbGlyphFont, FontVec, PxScale, ScaleFont};
use std::fs;

/// Noto Sans Mono (SIL OFL) — bundled for reliable Unicode coverage in PNG screenshots.
static EMBEDDED_FONT: &[u8] = include_bytes!("fonts/NotoSansMono-Regular.ttf");

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
    window: bool,
) -> Result<(), String> {
    if window {
        return screenshot_window(path, annotate, html, theme, session);
    }

    // Capture the ANSI content
    let ansi_content = snapshot::capture_ansi(session, None)
        .map_err(|e| format!("Failed to capture pane: {}", e))?;

    let (cols, rows, cursor_x, cursor_y) = snapshot::get_pane_info(session, None)
        .map_err(|e| format!("Failed to get pane info: {}", e))?;

    if html {
        let default_path = default_screenshot_path(session, "html");
        let output_path = path.unwrap_or(&default_path);
        let html_content = render_html(
            &ansi_content,
            cols,
            rows,
            cursor_x,
            cursor_y,
            annotate,
            theme,
        );
        fs::write(output_path, &html_content)
            .map_err(|e| format!("Failed to write HTML: {}", e))?;
        println!("Screenshot saved to {}", output_path);
    } else {
        let default_path = default_screenshot_path(session, "png");
        let output_path = path.unwrap_or(&default_path);
        render_png(
            &ansi_content,
            cols,
            rows,
            cursor_x,
            cursor_y,
            annotate,
            theme,
            output_path,
        )?;
        println!("Screenshot saved to {}", output_path);
    }

    Ok(())
}

/// Screenshot all panes in the window, composited at their layout positions.
fn screenshot_window(
    path: Option<&str>,
    annotate: bool,
    html: bool,
    theme: &str,
    session: &str,
) -> Result<(), String> {
    let panes = snapshot::list_pane_layouts(session)?;
    let (win_cols, win_rows) = snapshot::get_window_size(session)?;

    if panes.len() == 1 {
        return screenshot(path, annotate, html, theme, session, false);
    }

    let mut pane_data = Vec::new();
    for p in &panes {
        let ansi_content = snapshot::capture_ansi(session, Some(&p.pane_id))?;
        let (_cols, _rows, cx, cy) = snapshot::get_pane_info(session, Some(&p.pane_id))?;
        pane_data.push(PaneData {
            layout: p.clone(),
            ansi_content,
            cursor_x: cx,
            cursor_y: cy,
        });
    }

    if html {
        let default_path = default_screenshot_path(session, "html");
        let output_path = path.unwrap_or(&default_path);
        let html_content = render_window_html(&pane_data, win_cols, win_rows, annotate, theme);
        fs::write(output_path, &html_content)
            .map_err(|e| format!("Failed to write HTML: {}", e))?;
        println!("Screenshot saved to {}", output_path);
    } else {
        let default_path = default_screenshot_path(session, "png");
        let output_path = path.unwrap_or(&default_path);
        render_window_png(&pane_data, win_cols, win_rows, annotate, theme, output_path)?;
        println!("Screenshot saved to {}", output_path);
    }

    Ok(())
}

struct PaneData {
    layout: snapshot::PaneLayout,
    ansi_content: String,
    cursor_x: u16,
    cursor_y: u16,
}

fn render_window_html(
    pane_data: &[PaneData],
    win_cols: u16,
    win_rows: u16,
    annotate: bool,
    theme: &str,
) -> String {
    let (bg_color, fg_color) = if theme == "light" {
        ("#ffffff", "#000000")
    } else {
        ("#1e1e1e", "#d4d4d4")
    };

    let separator_color = if theme == "light" { "#ccc" } else { "#555" };
    let color_map = AnsiColorMap::new(theme);

    // Cell dimensions in px (must match the monospace font metrics)
    let cell_w_px = 8.4; // approx width of 14px monospace char
    let cell_h_px = 19.6; // line-height: 1.4 * 14px

    let total_w = (win_cols as f64) * cell_w_px;
    let total_h = (win_rows as f64) * cell_h_px;

    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str(&format!(
        "<title>agent-terminal window screenshot ({}x{}, {} panes)</title>\n",
        win_cols,
        win_rows,
        pane_data.len()
    ));
    html.push_str("<style>\n");
    html.push_str(&format!(
        "body {{ margin: 0; padding: 20px; background: {}; }}\n",
        bg_color
    ));
    html.push_str(".window-container {\n");
    html.push_str("  position: relative;\n");
    html.push_str(&format!("  width: {:.1}px;\n", total_w + 32.0)); // + padding
    html.push_str(&format!("  height: {:.1}px;\n", total_h + 60.0)); // + title + padding
    html.push_str(&format!("  background: {};\n", bg_color));
    html.push_str("  border: 1px solid #333;\n");
    html.push_str("  border-radius: 8px;\n");
    html.push_str("  overflow: hidden;\n");
    html.push_str("}\n");
    html.push_str(".title-bar {\n");
    html.push_str("  background: #333;\n");
    html.push_str("  color: #aaa;\n");
    html.push_str("  padding: 6px 16px;\n");
    html.push_str("  font-family: sans-serif;\n");
    html.push_str("  font-size: 12px;\n");
    html.push_str("  border-radius: 8px 8px 0 0;\n");
    html.push_str("}\n");
    html.push_str(".pane {\n");
    html.push_str("  position: absolute;\n");
    html.push_str("  overflow: hidden;\n");
    html.push_str(&format!("  color: {};\n", fg_color));
    html.push_str("  font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;\n");
    html.push_str("  font-size: 14px;\n");
    html.push_str("  line-height: 1.4;\n");
    html.push_str("  white-space: pre;\n");
    html.push_str("}\n");
    html.push_str(&format!(
        ".pane-separator {{ position: absolute; background: {}; }}\n",
        separator_color
    ));
    html.push_str(".bold { font-weight: bold; }\n");
    html.push_str(".dim { opacity: 0.5; }\n");
    html.push_str(".italic { font-style: italic; }\n");
    html.push_str(".underline { text-decoration: underline; }\n");
    html.push_str(".strikethrough { text-decoration: line-through; }\n");
    html.push_str(".cursor { background: #ffffff40; outline: 1px solid #fff; }\n");
    if annotate {
        html.push_str(".pane-label { position: absolute; background: #333; color: #aaa; font-family: sans-serif; font-size: 10px; padding: 1px 4px; border-radius: 2px; z-index: 10; }\n");
    }
    html.push_str("</style>\n</head>\n<body>\n");

    html.push_str(&format!(
        "<div class=\"title-bar\">agent-terminal window — {}x{} — {} panes</div>\n",
        win_cols,
        win_rows,
        pane_data.len()
    ));

    let content_top = 28.0; // title bar height
    let pad = 16.0;
    html.push_str("<div class=\"window-container\">\n");

    // Draw separator lines between panes
    // Build a mask to find separator cells
    let mut pane_mask = vec![vec![false; win_cols as usize]; win_rows as usize];
    for pd in pane_data {
        let p = &pd.layout;
        for row in p.top..(p.top + p.height).min(win_rows) {
            for col in p.left..(p.left + p.width).min(win_cols) {
                pane_mask[row as usize][col as usize] = true;
            }
        }
    }

    // Draw vertical separators
    #[allow(clippy::needless_range_loop)]
    for col in 0..win_cols as usize {
        let mut in_sep = false;
        let mut sep_start = 0;
        for row in 0..win_rows as usize {
            if !pane_mask[row][col] {
                if !in_sep {
                    in_sep = true;
                    sep_start = row;
                }
            } else if in_sep {
                let x = (col as f64) * cell_w_px + pad;
                let y = (sep_start as f64) * cell_h_px + content_top + pad;
                let h = ((row - sep_start) as f64) * cell_h_px;
                html.push_str(&format!(
                    "<div class=\"pane-separator\" style=\"left:{:.1}px;top:{:.1}px;width:1px;height:{:.1}px;\"></div>\n",
                    x, y, h
                ));
                in_sep = false;
            }
        }
    }

    // Render each pane
    for pd in pane_data {
        let p = &pd.layout;
        let x = (p.left as f64) * cell_w_px + pad;
        let y = (p.top as f64) * cell_h_px + content_top + pad;
        let w = (p.width as f64) * cell_w_px;
        let h = (p.height as f64) * cell_h_px;

        if annotate {
            html.push_str(&format!(
                "<div class=\"pane-label\" style=\"left:{:.1}px;top:{:.1}px;\">{}{}</div>\n",
                x,
                y - 14.0,
                pd.layout.pane_id,
                if pd.layout.active { " *" } else { "" }
            ));
        }

        html.push_str(&format!(
            "<div class=\"pane\" style=\"left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px;\">\n",
            x, y, w, h
        ));

        let lines: Vec<&str> = pd.ansi_content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let html_line = ansi_line_to_html(
                line,
                &color_map,
                i,
                pd.cursor_x as usize,
                pd.cursor_y as usize,
            );
            html.push_str(&html_line);
            html.push('\n');
        }

        html.push_str("</div>\n");
    }

    html.push_str("</div>\n</body>\n</html>");
    html
}

fn render_window_png(
    pane_data: &[PaneData],
    win_cols: u16,
    win_rows: u16,
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
    let descent = -scaled.descent();
    let cell_height = (ascent + descent).ceil() as u32;

    let padding: u32 = 16;
    let title_bar_height: u32 = 28;

    let img_width = padding * 2 + (win_cols as u32 * cell_width);
    let img_height = padding * 2 + title_bar_height + (win_rows as u32 * cell_height);

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

    let separator_color = (80u8, 80u8, 80u8);

    let mut imgbuf =
        image::RgbaImage::from_pixel(img_width, img_height, image::Rgba([bg_r, bg_g, bg_b, 255]));

    // Draw title bar
    for y in 0..title_bar_height {
        for x in 0..img_width {
            imgbuf.put_pixel(x, y, image::Rgba([51, 51, 51, 255]));
        }
    }

    let y_offset = title_bar_height + padding;
    let x_offset = padding;

    // Draw separator lines (cells not belonging to any pane)
    let mut pane_mask = vec![vec![false; win_cols as usize]; win_rows as usize];
    for pd in pane_data {
        let p = &pd.layout;
        for row in p.top..(p.top + p.height).min(win_rows) {
            for col in p.left..(p.left + p.width).min(win_cols) {
                pane_mask[row as usize][col as usize] = true;
            }
        }
    }

    #[allow(clippy::needless_range_loop)]
    for row in 0..win_rows as usize {
        for col in 0..win_cols as usize {
            if !pane_mask[row][col] {
                let px_x = x_offset + (col as u32 * cell_width);
                let px_y = y_offset + (row as u32 * cell_height);
                // Draw a thin separator line
                let mid_x = px_x + cell_width / 2;
                for dy in 0..cell_height {
                    if mid_x < img_width && px_y + dy < img_height {
                        imgbuf.put_pixel(
                            mid_x,
                            px_y + dy,
                            image::Rgba([
                                separator_color.0,
                                separator_color.1,
                                separator_color.2,
                                255,
                            ]),
                        );
                    }
                }
            }
        }
    }

    // Render each pane's content
    for pd in pane_data {
        let p = &pd.layout;
        let pane_x = x_offset + (p.left as u32 * cell_width);
        let pane_y = y_offset + (p.top as u32 * cell_height);

        let lines: Vec<&str> = pd.ansi_content.lines().collect();
        let mut style = StyleState::default();

        for (line_idx, line) in lines.iter().enumerate() {
            let row_y = pane_y + (line_idx as u32 * cell_height);
            let baseline_y = row_y as f32 + ascent;

            if annotate && line_idx == 0 {
                // Draw pane ID label
                let label = p.pane_id.to_string();
                for (i, ch) in label.chars().enumerate() {
                    let gx = pane_x + (i as u32 * cell_width);
                    // Small label in top-right area would overlap; just skip for PNG
                    let _ = (gx, ch); // pane labels are more practical in HTML
                }
            }

            let cells = parse_ansi_line_to_cells(line, default_fg, &mut style);
            for (col_idx, cell) in cells.iter().enumerate() {
                let cx = pane_x + (col_idx as u32 * cell_width);
                let cy = row_y;

                if let Some((br, bg, bb)) = cell.bg {
                    for dy in 0..cell_height {
                        for dx in 0..cell_width {
                            if cx + dx < img_width && cy + dy < img_height {
                                imgbuf.put_pixel(cx + dx, cy + dy, image::Rgba([br, bg, bb, 255]));
                            }
                        }
                    }
                }

                if cell.ch != ' ' {
                    draw_glyph(
                        &mut imgbuf,
                        &font,
                        cell.ch,
                        scale,
                        cx as f32,
                        baseline_y,
                        cell.fg,
                        cell_width,
                        cell_height,
                    );
                }
            }

            // Draw cursor
            if line_idx == pd.cursor_y as usize {
                let cx = pane_x + (pd.cursor_x as u32 * cell_width);
                let cy = row_y;
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
    }

    imgbuf
        .save(output_path)
        .map_err(|e| format!("Failed to save PNG: {}", e))?;

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
            html.push_str(&format!("<span class=\"row-num\">{:>3}│</span>", i + 1));
        }

        // Parse ANSI in this line and convert to HTML spans
        let html_line =
            ansi_line_to_html(line, &color_map, i, cursor_x as usize, cursor_y as usize);
        html.push_str(&html_line);
        html.push('\n');
    }

    html.push_str("</div>\n</body>\n</html>");
    html
}

/// Convert a single ANSI line to HTML spans.
fn ansi_line_to_html(
    line: &str,
    color_map: &AnsiColorMap,
    row: usize,
    cursor_x: usize,
    cursor_y: usize,
) -> String {
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
            let is_cursor = row == cursor_y && col == cursor_x;
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
fn parse_ansi_line_to_cells(
    line: &str,
    default_fg: (u8, u8, u8),
    style: &mut StyleState,
) -> Vec<ColoredCell> {
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

/// Load a monospace font — tries the bundled Noto Sans Mono first, then system fonts.
fn load_font() -> Result<FontVec, String> {
    // 1. Try embedded Noto Sans Mono (bundled, works everywhere)
    if let Ok(font) = FontVec::try_from_vec(EMBEDDED_FONT.to_vec()) {
        return Ok(font);
    }

    // 2. Fallback: system fonts
    let candidates: &[(&str, Option<u32>)] = &[
        // macOS
        ("/System/Library/Fonts/Menlo.ttc", Some(0)),
        ("/System/Library/Fonts/Monaco.ttf", None),
        ("/System/Library/Fonts/Supplemental/Courier New.ttf", None),
        // Linux
        ("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", None),
        (
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            None,
        ),
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

    Err(
        "No monospace font found. Bundled font failed to parse, and no system fonts available."
            .into(),
    )
}

/// Draw a single glyph onto the image with alpha blending.
/// When the font lacks a glyph, draws a tofu box (rectangular outline) as a visible placeholder.
#[allow(clippy::too_many_arguments)]
fn draw_glyph(
    img: &mut image::RgbaImage,
    font: &FontVec,
    ch: char,
    scale: PxScale,
    x: f32,
    baseline_y: f32,
    color: (u8, u8, u8),
    cell_width: u32,
    cell_height: u32,
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
    } else {
        // Missing glyph — draw a tofu box (1px rectangular outline) as a visible placeholder
        draw_tofu_box(
            img,
            x as u32,
            baseline_y as u32 - cell_height + cell_height / 4,
            cell_width,
            cell_height * 3 / 4,
            color,
        );
    }
}

/// Draw a 1px rectangular outline (tofu box) to indicate a missing glyph.
fn draw_tofu_box(
    img: &mut image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: (u8, u8, u8),
) {
    let (r, g, b) = color;
    let pixel = image::Rgba([r, g, b, 255]);
    let img_w = img.width();
    let img_h = img.height();

    // Inset by 1px on left/right so adjacent tofu boxes don't merge
    let x0 = x + 1;
    let x1 = (x + width).saturating_sub(1);
    let y0 = y;
    let y1 = y + height;

    // Top and bottom edges
    for px in x0..x1 {
        if px < img_w {
            if y0 < img_h {
                img.put_pixel(px, y0, pixel);
            }
            if y1 < img_h {
                img.put_pixel(px, y1, pixel);
            }
        }
    }
    // Left and right edges
    for py in y0..=y1 {
        if py < img_h {
            if x0 < img_w {
                img.put_pixel(x0, py, pixel);
            }
            if x1 < img_w {
                img.put_pixel(x1, py, pixel);
            }
        }
    }
}

/// Render terminal content as PNG using ab_glyph for real font rendering.
#[allow(clippy::too_many_arguments)]
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

    let mut imgbuf =
        image::RgbaImage::from_pixel(img_width, img_height, image::Rgba([bg_r, bg_g, bg_b, 255]));

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
                draw_glyph(
                    &mut imgbuf,
                    &font,
                    ch,
                    scale,
                    gx as f32,
                    baseline_y,
                    gutter_color,
                    cell_width,
                    cell_height,
                );
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
                draw_glyph(
                    &mut imgbuf,
                    &font,
                    cell.ch,
                    scale,
                    cx as f32,
                    baseline_y,
                    cell.fg,
                    cell_width,
                    cell_height,
                );
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

        let nums: Vec<u32> = params.split(';').filter_map(|s| s.parse().ok()).collect();

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
                22 => {
                    self.bold = false;
                    self.dim = false;
                }
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
        1 => (205, 49, 49),   // red
        2 => (13, 188, 121),  // green
        3 => (229, 229, 16),  // yellow
        4 => (36, 114, 200),  // blue
        5 => (188, 63, 188),  // magenta
        6 => (17, 168, 205),  // cyan
        7 => (229, 229, 229), // white
        _ => (229, 229, 229),
    }
}

fn ansi_bright_color(idx: u32) -> (u8, u8, u8) {
    match idx {
        0 => (102, 102, 102), // bright black
        1 => (241, 76, 76),   // bright red
        2 => (35, 209, 139),  // bright green
        3 => (245, 245, 67),  // bright yellow
        4 => (59, 142, 234),  // bright blue
        5 => (214, 112, 214), // bright magenta
        6 => (41, 184, 219),  // bright cyan
        7 => (255, 255, 255), // bright white
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
