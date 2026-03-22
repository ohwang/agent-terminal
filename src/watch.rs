use std::io::stdout;
use std::process::Command;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style as RStyle};
use ratatui::text::{Line as RLine, Span as RSpan};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;

use crate::snapshot;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum SessionStatus {
    Live,
    Ended,
}

#[derive(Clone)]
struct TrackedSession {
    name: String,
    created_ts: i64,
    last_content: String,
    last_ansi_content: String,
    status: SessionStatus,
}

struct App {
    sessions: Vec<TrackedSession>,
    selected: usize,
    zoomed: bool,
    filter: Option<String>,
    poll_interval: Duration,
    last_poll: Instant,
    scroll_offset: u16,
}

impl App {
    fn new(interval: u64, filter: Option<&str>) -> Self {
        Self {
            sessions: Vec::new(),
            selected: 0,
            zoomed: false,
            filter: filter.map(|s| s.to_string()),
            poll_interval: Duration::from_millis(interval),
            last_poll: Instant::now() - Duration::from_secs(10), // force immediate poll
            scroll_offset: 0,
        }
    }

    fn poll_sessions(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_poll) < self.effective_interval() {
            return;
        }
        self.last_poll = now;

        let live_sessions = list_tmux_sessions();

        // Filter if requested
        let live_sessions: Vec<(String, i64)> = if let Some(ref prefix) = self.filter {
            live_sessions
                .into_iter()
                .filter(|(name, _)| name.starts_with(prefix.as_str()))
                .collect()
        } else {
            live_sessions
        };

        let live_names: Vec<&str> = live_sessions.iter().map(|(n, _)| n.as_str()).collect();

        // Mark disappeared sessions as ended
        for session in &mut self.sessions {
            if matches!(session.status, SessionStatus::Live)
                && !live_names.contains(&session.name.as_str())
            {
                session.status = SessionStatus::Ended;
            }
        }

        // Add new sessions, revive re-appeared ones
        for (name, ts) in &live_sessions {
            if let Some(existing) = self.sessions.iter_mut().find(|s| s.name == *name) {
                if existing.created_ts != *ts {
                    // Same name but different created timestamp — new instance
                    existing.created_ts = *ts;
                    existing.status = SessionStatus::Live;
                    existing.last_content.clear();
                    existing.last_ansi_content.clear();
                } else if matches!(existing.status, SessionStatus::Ended) {
                    existing.status = SessionStatus::Live;
                }
            } else {
                self.sessions.push(TrackedSession {
                    name: name.clone(),
                    created_ts: *ts,
                    last_content: String::new(),
                    last_ansi_content: String::new(),
                    status: SessionStatus::Live,
                });
            }
        }

        // Capture content for live sessions (always with ANSI for color rendering)
        for session in &mut self.sessions {
            if matches!(session.status, SessionStatus::Ended) {
                continue;
            }
            if let Ok(ansi) = snapshot::capture_ansi(&session.name, None) {
                session.last_ansi_content = ansi.clone();
                // Strip ANSI for plain fallback
                session.last_content = snapshot::parse_ansi(&ansi)
                    .into_iter()
                    .map(|(text, _)| text)
                    .collect();
            } else if let Ok(content) = snapshot::capture_plain(&session.name, None) {
                session.last_content = content;
            }
        }

        // Clamp selection
        if !self.sessions.is_empty() && self.selected >= self.sessions.len() {
            self.selected = self.sessions.len() - 1;
        }
    }

    fn effective_interval(&self) -> Duration {
        if self.zoomed {
            // Poll faster when zoomed
            Duration::from_millis(100)
        } else {
            self.poll_interval
        }
    }

    fn selected_session(&self) -> Option<&TrackedSession> {
        self.sessions.get(self.selected)
    }

    fn select_next(&mut self) {
        if !self.sessions.is_empty() {
            self.selected = (self.selected + 1) % self.sessions.len();
            self.scroll_offset = 0;
        }
    }

    fn select_prev(&mut self) {
        if !self.sessions.is_empty() {
            self.selected = if self.selected == 0 {
                self.sessions.len() - 1
            } else {
                self.selected - 1
            };
            self.scroll_offset = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Tmux helpers
// ---------------------------------------------------------------------------

fn list_tmux_sessions() -> Vec<(String, i64)> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name} #{session_created}"])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut sessions = Vec::new();
    for line in text.trim().lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let name = parts[0].to_string();
            let ts = parts[1].parse::<i64>().unwrap_or(0);
            sessions.push((name, ts));
        }
    }
    sessions
}

fn format_age(created_ts: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let diff = now - created_ts;
    if diff < 60 {
        format!("{}s", diff)
    } else if diff < 3600 {
        format!("{}m", diff / 60)
    } else {
        format!("{}h{}m", diff / 3600, (diff % 3600) / 60)
    }
}

// ---------------------------------------------------------------------------
// ANSI -> ratatui style conversion
// ---------------------------------------------------------------------------

fn color_name_to_ratatui(name: &str) -> Color {
    match name {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "bright-black" => Color::DarkGray,
        "bright-red" => Color::LightRed,
        "bright-green" => Color::LightGreen,
        "bright-yellow" => Color::LightYellow,
        "bright-blue" => Color::LightBlue,
        "bright-magenta" => Color::LightMagenta,
        "bright-cyan" => Color::LightCyan,
        "bright-white" => Color::Gray,
        s if s.starts_with('#') && s.len() == 7 => {
            let r = u8::from_str_radix(&s[1..3], 16).unwrap_or(0);
            let g = u8::from_str_radix(&s[3..5], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[5..7], 16).unwrap_or(0);
            Color::Rgb(r, g, b)
        }
        s => {
            if let Ok(idx) = s.parse::<u8>() {
                Color::Indexed(idx)
            } else {
                Color::Reset
            }
        }
    }
}

fn snapshot_style_to_ratatui(style: &snapshot::Style) -> RStyle {
    let mut rs = RStyle::default();
    if let Some(ref fg) = style.fg {
        rs = rs.fg(color_name_to_ratatui(fg));
    }
    if let Some(ref bg) = style.bg {
        rs = rs.bg(color_name_to_ratatui(bg));
    }
    let mut mods = Modifier::empty();
    if style.bold {
        mods |= Modifier::BOLD;
    }
    if style.dim {
        mods |= Modifier::DIM;
    }
    if style.italic {
        mods |= Modifier::ITALIC;
    }
    if style.underline {
        mods |= Modifier::UNDERLINED;
    }
    if style.blink {
        mods |= Modifier::SLOW_BLINK;
    }
    if style.reverse {
        mods |= Modifier::REVERSED;
    }
    if style.strikethrough {
        mods |= Modifier::CROSSED_OUT;
    }
    rs.add_modifier(mods)
}

fn ansi_to_ratatui_lines(ansi_content: &str) -> Vec<RLine<'static>> {
    ansi_content
        .lines()
        .map(|line| {
            let segments = snapshot::parse_ansi(line);
            let spans: Vec<RSpan<'static>> = segments
                .into_iter()
                .map(|(text, style)| RSpan::styled(text, snapshot_style_to_ratatui(&style)))
                .collect();
            RLine::from(spans)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn grid_dimensions(count: usize, area: Rect) -> (usize, usize) {
    if count == 0 {
        return (1, 1);
    }
    let cols = if count == 1 {
        1
    } else if count <= 4 {
        2
    } else if count <= 9 {
        3
    } else {
        4
    };
    let rows = (count + cols - 1) / cols;
    // Limit rows to what fits on screen (minimum ~6 rows per cell)
    let max_rows = (area.height as usize / 6).max(1);
    let rows = rows.min(max_rows);
    (cols, rows)
}

fn render_grid(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    if app.sessions.is_empty() {
        let msg = Paragraph::new("No sessions found. Waiting...")
            .style(RStyle::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" agent-terminal watch "),
            );
        f.render_widget(msg, area);
        return;
    }

    // Reserve 1 row for the status bar at the bottom
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    let grid_area = chunks[0];
    let status_area = chunks[1];

    let count = app.sessions.len();
    let (cols, rows) = grid_dimensions(count, grid_area);

    // Split vertically into rows
    let row_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Ratio(1, rows as u32))
        .collect();
    let row_chunks = Layout::vertical(row_constraints).split(grid_area);

    for row_idx in 0..rows {
        // Split each row horizontally into columns
        let col_constraints: Vec<Constraint> = (0..cols)
            .map(|_| Constraint::Ratio(1, cols as u32))
            .collect();
        let col_chunks = Layout::horizontal(col_constraints).split(row_chunks[row_idx]);

        for col_idx in 0..cols {
            let session_idx = row_idx * cols + col_idx;
            if session_idx >= count {
                // Empty cell
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(RStyle::default().fg(Color::DarkGray));
                f.render_widget(block, col_chunks[col_idx]);
                continue;
            }

            let session = &app.sessions[session_idx];
            let is_selected = session_idx == app.selected;

            let (status_label, status_color) = match session.status {
                SessionStatus::Live => ("live", Color::Green),
                SessionStatus::Ended => ("ENDED", Color::Red),
            };

            let title = format!(
                " {} | {} | {} ",
                session.name,
                format_age(session.created_ts),
                status_label
            );

            let border_color = if is_selected {
                Color::Cyan
            } else {
                match session.status {
                    SessionStatus::Live => Color::White,
                    SessionStatus::Ended => Color::DarkGray,
                }
            };

            let border_style = RStyle::default().fg(border_color);
            let title_style = RStyle::default().fg(status_color);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(RLine::from(vec![RSpan::styled(title, title_style)]));

            let is_ended = matches!(session.status, SessionStatus::Ended);

            let paragraph = if !session.last_ansi_content.is_empty() && !is_ended {
                let lines = ansi_to_ratatui_lines(&session.last_ansi_content);
                Paragraph::new(lines).block(block)
            } else if !session.last_content.is_empty() {
                let style = if is_ended {
                    RStyle::default().fg(Color::DarkGray)
                } else {
                    RStyle::default()
                };
                Paragraph::new(session.last_content.clone())
                    .style(style)
                    .block(block)
            } else {
                let placeholder = match session.status {
                    SessionStatus::Live => "(waiting for content...)",
                    SessionStatus::Ended => "(no content captured)",
                };
                Paragraph::new(placeholder)
                    .style(RStyle::default().fg(Color::DarkGray))
                    .block(block)
            };
            f.render_widget(paragraph, col_chunks[col_idx]);
        }
    }

    // Status bar
    let help = " [q] Quit  [Enter/z] Zoom  [j/k] Navigate  [Tab] Next ";
    let status = Paragraph::new(help).style(RStyle::default().fg(Color::DarkGray));
    f.render_widget(status, status_area);
}

fn render_zoomed(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let session = match app.selected_session() {
        Some(s) => s,
        None => {
            let msg = Paragraph::new("No session selected.");
            f.render_widget(msg, area);
            return;
        }
    };

    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    let content_area = chunks[0];
    let status_area = chunks[1];

    let (status_label, status_color) = match session.status {
        SessionStatus::Live => ("live", Color::Green),
        SessionStatus::Ended => ("ENDED", Color::Red),
    };

    let title = format!(
        " {} | {} | {} ",
        session.name,
        format_age(session.created_ts),
        status_label
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(RLine::from(vec![RSpan::styled(
            title,
            RStyle::default().fg(status_color),
        )]));

    let inner = block.inner(content_area);

    // Use ANSI content if available for color rendering, else fall back to plain
    let paragraph = if !session.last_ansi_content.is_empty() {
        let lines = ansi_to_ratatui_lines(&session.last_ansi_content);
        let total_lines = lines.len() as u16;
        let visible = inner.height;
        let max_scroll = total_lines.saturating_sub(visible);
        let scroll = app.scroll_offset.min(max_scroll);
        Paragraph::new(lines).block(block).scroll((scroll, 0))
    } else {
        let total_lines = session.last_content.lines().count() as u16;
        let visible = inner.height;
        let max_scroll = total_lines.saturating_sub(visible);
        let scroll = app.scroll_offset.min(max_scroll);
        Paragraph::new(session.last_content.clone())
            .block(block)
            .scroll((scroll, 0))
    };
    f.render_widget(paragraph, content_area);

    // Status bar
    let help = format!(
        " [Esc] Back to grid  [j/k] Scroll  [q] Quit  |  tmux attach -t {} -r ",
        session.name
    );
    let status = Paragraph::new(help).style(RStyle::default().fg(Color::DarkGray));
    f.render_widget(status, status_area);
}

// ---------------------------------------------------------------------------
// Main entry
// ---------------------------------------------------------------------------

pub fn run(interval: u64, filter: Option<&str>) -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
    stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| format!("Failed to enter alternate screen: {e}"))?;

    let result = run_inner(interval, filter);

    // Always restore terminal
    let _ = disable_raw_mode();
    let _ = stdout().execute(LeaveAlternateScreen);

    result
}

fn run_inner(interval: u64, filter: Option<&str>) -> Result<(), String> {
    let backend = CrosstermBackend::new(stdout());
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {e}"))?;

    let mut app = App::new(interval, filter);

    loop {
        app.poll_sessions();

        terminal
            .draw(|f| {
                if app.zoomed {
                    render_zoomed(f, &app);
                } else {
                    render_grid(f, &app);
                }
            })
            .map_err(|e| format!("Draw error: {e}"))?;

        // Wait for events up to the remaining poll interval
        let elapsed = app.last_poll.elapsed();
        let remaining = app.effective_interval().saturating_sub(elapsed);
        let timeout = remaining.max(Duration::from_millis(16)); // minimum ~60fps render

        if event::poll(timeout).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Esc => {
                        if app.zoomed {
                            app.zoomed = false;
                            app.scroll_offset = 0;
                        } else {
                            break;
                        }
                    }
                    KeyCode::Enter | KeyCode::Char('z') if !app.zoomed => {
                        if !app.sessions.is_empty() {
                            app.zoomed = true;
                            app.scroll_offset = 0;
                            // Force immediate re-poll with ANSI capture
                            app.last_poll =
                                Instant::now() - Duration::from_secs(10);
                        }
                    }
                    KeyCode::Tab => {
                        if !app.zoomed {
                            app.select_next();
                        }
                    }
                    KeyCode::BackTab => {
                        if !app.zoomed {
                            app.select_prev();
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app.zoomed {
                            app.scroll_offset = app.scroll_offset.saturating_add(1);
                        } else {
                            app.select_next();
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.zoomed {
                            app.scroll_offset = app.scroll_offset.saturating_sub(1);
                        } else {
                            app.select_prev();
                        }
                    }
                    KeyCode::Char('h') | KeyCode::Left if !app.zoomed => {
                        app.select_prev();
                    }
                    KeyCode::Char('l') | KeyCode::Right if !app.zoomed => {
                        app.select_next();
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
