mod session;
mod snapshot;
mod interact;
mod wait;
mod annotate;
mod perf;
mod watch;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "agent-terminal", version, about = "TUI testing tool for autonomous agent-driven terminal application testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch a command in a new tmux session
    Open {
        /// Command to run
        command: String,
        /// Session name (default: agent-terminal)
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Named pane within the session
        #[arg(long)]
        pane: Option<String>,
        /// Environment variables (KEY=VAL)
        #[arg(long = "env", num_args = 1)]
        envs: Vec<String>,
        /// Initial terminal size (COLSxROWS)
        #[arg(long)]
        size: Option<String>,
        /// Keep session alive after command exits (wraps in shell)
        #[arg(long)]
        shell: bool,
        /// Don't capture stderr (needed for bash/readline apps)
        #[arg(long)]
        no_stderr: bool,
    },
    /// Kill a tmux session
    Close {
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// List active sessions
    List,
    /// Get process status
    Status {
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Pane name
        #[arg(long)]
        pane: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Get exit code of the process
    ExitCode {
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Capture stderr/stdout logs
    Logs {
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Show stderr only
        #[arg(long)]
        stderr: bool,
    },
    /// Capture a snapshot of the terminal
    Snapshot {
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Pane name
        #[arg(long)]
        pane: Option<String>,
        /// Show color/style annotations
        #[arg(long)]
        color: bool,
        /// Raw byte stream (no formatting)
        #[arg(long)]
        raw: bool,
        /// Raw ANSI with row numbers
        #[arg(long)]
        ansi: bool,
        /// Structured JSON output
        #[arg(long)]
        json: bool,
        /// Diff against last snapshot
        #[arg(long)]
        diff: bool,
        /// Include N lines of scrollback
        #[arg(long)]
        scrollback: Option<usize>,
    },
    /// Send key sequences
    Send {
        /// Keys to send (e.g., "j", "Enter", "C-c")
        #[arg(num_args = 1..)]
        keys: Vec<String>,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Pane name
        #[arg(long)]
        pane: Option<String>,
    },
    /// Type literal text
    Type {
        /// Text to type
        text: String,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Pane name
        #[arg(long)]
        pane: Option<String>,
    },
    /// Paste text via tmux paste buffer
    Paste {
        /// Text to paste
        text: String,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Pane name
        #[arg(long)]
        pane: Option<String>,
    },
    /// Resize the terminal
    Resize {
        /// Number of columns
        cols: u16,
        /// Number of rows
        rows: u16,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Pane name
        #[arg(long)]
        pane: Option<String>,
    },
    /// Click at a position
    Click {
        /// Row (1-indexed)
        row: u16,
        /// Column (1-indexed)
        col: u16,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Right click
        #[arg(long)]
        right: bool,
        /// Double click
        #[arg(long)]
        double: bool,
    },
    /// Drag from one position to another
    Drag {
        /// Start row
        r1: u16,
        /// Start column
        c1: u16,
        /// End row
        r2: u16,
        /// End column
        c2: u16,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Scroll wheel at position
    ScrollWheel {
        /// Direction (up/down)
        direction: String,
        /// Row
        row: u16,
        /// Column
        col: u16,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Wait for a condition
    Wait {
        /// Hard wait in milliseconds
        ms: Option<u64>,
        /// Wait until text appears
        #[arg(long)]
        text: Option<String>,
        /// Wait until text disappears
        #[arg(long = "text-gone")]
        text_gone: Option<String>,
        /// Wait until screen stable for N ms
        #[arg(long)]
        stable: Option<u64>,
        /// Wait until cursor at row,col
        #[arg(long)]
        cursor: Option<String>,
        /// Wait for regex match
        #[arg(long)]
        regex: Option<String>,
        /// Wait until process exits
        #[arg(long)]
        exit: bool,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
        /// Timeout in ms (default: 10000)
        #[arg(long, default_value = "10000")]
        timeout: u64,
        /// Poll interval in ms (default: 50)
        #[arg(long, default_value = "50")]
        interval: u64,
    },
    /// Assert a condition (exit 0 if pass, exit 1 if fail)
    Assert {
        /// Assert text is present
        #[arg(long)]
        text: Option<String>,
        /// Assert text is absent
        #[arg(long = "no-text")]
        no_text: Option<String>,
        /// Assert row contains text (row_num)
        #[arg(long)]
        row: Option<u16>,
        /// Text to check in specified row
        #[arg(long = "row-text")]
        row_text: Option<String>,
        /// Assert cursor on row
        #[arg(long = "cursor-row")]
        cursor_row: Option<u16>,
        /// Assert color style on row (row_num)
        #[arg(long)]
        color: Option<u16>,
        /// Style string to check (e.g., "fg:red,bold")
        #[arg(long = "color-style")]
        color_style: Option<String>,
        /// Assert text has a specific style
        #[arg(long)]
        style: Option<String>,
        /// Style to check for --style text
        #[arg(long = "style-check")]
        style_check: Option<String>,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Find text on screen
    Find {
        /// Text to find
        pattern: String,
        /// Return all matches
        #[arg(long)]
        all: bool,
        /// Use regex
        #[arg(long)]
        regex: bool,
        /// Find by color
        #[arg(long)]
        color: Option<String>,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Capture screenshot as image
    Screenshot {
        /// Output path
        #[arg(long)]
        path: Option<String>,
        /// Overlay row/col grid
        #[arg(long)]
        annotate: bool,
        /// Save as HTML instead of PNG
        #[arg(long)]
        html: bool,
        /// Theme (dark/light)
        #[arg(long, default_value = "dark")]
        theme: String,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Send a signal to the process
    Signal {
        /// Signal name (SIGINT, SIGTERM, etc.)
        signal: String,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Clipboard operations
    Clipboard {
        /// Operation: read, write, paste
        operation: String,
        /// Text for write operation
        text: Option<String>,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Read tmux scrollback buffer
    Scrollback {
        /// Number of lines
        #[arg(long)]
        lines: Option<usize>,
        /// Search scrollback for text
        #[arg(long)]
        search: Option<String>,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Performance measurement
    Perf {
        #[command(subcommand)]
        command: PerfCommands,
    },
    /// Validate environment
    Doctor,
    /// Initialize project with starter test
    Init,
    /// Run tests across multiple configurations
    TestMatrix {
        /// Command to test
        #[arg(long)]
        command: String,
        /// Terminal sizes (e.g., "80x24,120x40,40x10")
        #[arg(long)]
        sizes: Option<String>,
        /// TERM values (e.g., "xterm-256color,screen-256color,dumb")
        #[arg(long)]
        terms: Option<String>,
        /// Color modes (e.g., "default,NO_COLOR=1,COLORTERM=truecolor")
        #[arg(long)]
        colors: Option<String>,
        /// Test commands to run after app starts
        #[arg(long)]
        test: String,
    },
    /// Accessibility check
    A11yCheck {
        /// Command to test
        command: String,
    },
    /// Live dashboard for observing all agent-terminal sessions
    Watch {
        /// Poll interval in milliseconds
        #[arg(long, default_value = "200")]
        interval: u64,
        /// Only show sessions matching this prefix
        #[arg(long)]
        filter: Option<String>,
    },
}

#[derive(Subcommand)]
enum PerfCommands {
    /// Start frame recording
    Start {
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Stop frame recording and return metrics
    Stop {
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Measure FPS
    Fps {
        /// Run command while measuring
        #[arg(long)]
        during: Option<String>,
        /// Read batch commands from stdin
        #[arg(long = "during-batch")]
        during_batch: bool,
        /// Observe for N ms without actions
        #[arg(long)]
        duration: Option<u64>,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
    /// Measure input latency
    Latency {
        /// Key to test
        #[arg(long)]
        key: Option<String>,
        /// Number of samples
        #[arg(long, default_value = "5")]
        samples: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Session name
        #[arg(long, default_value = "agent-terminal")]
        session: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Open { command, session, pane, envs, size, shell, no_stderr } => {
            session::open(&command, &session, pane.as_deref(), &envs, size.as_deref(), shell, no_stderr)
        }
        Commands::Close { session } => {
            session::close(&session)
        }
        Commands::List => {
            session::list()
        }
        Commands::Status { session, pane, json } => {
            session::status(&session, pane.as_deref(), json)
        }
        Commands::ExitCode { session } => {
            session::exit_code(&session)
        }
        Commands::Logs { session, stderr } => {
            session::logs(&session, stderr)
        }
        Commands::Snapshot { session, pane, color, raw, ansi, json, diff, scrollback } => {
            snapshot::snapshot(&session, pane.as_deref(), color, raw, ansi, json, diff, scrollback)
        }
        Commands::Send { keys, session, pane } => {
            interact::send_keys(&keys, &session, pane.as_deref())
        }
        Commands::Type { text, session, pane } => {
            interact::type_text(&text, &session, pane.as_deref())
        }
        Commands::Paste { text, session, pane } => {
            interact::paste(&text, &session, pane.as_deref())
        }
        Commands::Resize { cols, rows, session, pane } => {
            interact::resize(cols, rows, &session, pane.as_deref())
        }
        Commands::Click { row, col, session, right, double } => {
            interact::click(row, col, &session, right, double)
        }
        Commands::Drag { r1, c1, r2, c2, session } => {
            interact::drag(r1, c1, r2, c2, &session)
        }
        Commands::ScrollWheel { direction, row, col, session } => {
            interact::scroll_wheel(&direction, row, col, &session)
        }
        Commands::Wait { ms, text, text_gone, stable, cursor, regex, exit, session, timeout, interval } => {
            wait::wait(ms, text.as_deref(), text_gone.as_deref(), stable, cursor.as_deref(), regex.as_deref(), exit, &session, timeout, interval)
        }
        Commands::Assert { text, no_text, row, row_text, cursor_row, color, color_style, style, style_check, session } => {
            wait::assert_cmd(text.as_deref(), no_text.as_deref(), row, row_text.as_deref(), cursor_row, color, color_style.as_deref(), style.as_deref(), style_check.as_deref(), &session)
        }
        Commands::Find { pattern, all, regex, color, session } => {
            wait::find(&pattern, all, regex, color.as_deref(), &session)
        }
        Commands::Screenshot { path, annotate, html, theme, session } => {
            annotate::screenshot(path.as_deref(), annotate, html, &theme, &session)
        }
        Commands::Signal { signal, session } => {
            interact::signal(&signal, &session)
        }
        Commands::Clipboard { operation, text, session } => {
            interact::clipboard(&operation, text.as_deref(), &session)
        }
        Commands::Scrollback { lines, search, session } => {
            snapshot::scrollback_cmd(lines, search.as_deref(), &session)
        }
        Commands::Perf { command } => {
            match command {
                PerfCommands::Start { session } => perf::start(&session),
                PerfCommands::Stop { json, session } => perf::stop(json, &session),
                PerfCommands::Fps { during, during_batch, duration, session } => {
                    perf::fps(during.as_deref(), during_batch, duration, &session)
                }
                PerfCommands::Latency { key, samples, json, session } => {
                    perf::latency(key.as_deref(), samples, json, &session)
                }
            }
        }
        Commands::Doctor => {
            session::doctor()
        }
        Commands::Init => {
            session::init()
        }
        Commands::TestMatrix { command, sizes, terms, colors, test } => {
            session::test_matrix(&command, sizes.as_deref(), terms.as_deref(), colors.as_deref(), &test)
        }
        Commands::A11yCheck { command } => {
            session::a11y_check(&command)
        }
        Commands::Watch { interval, filter } => {
            watch::run(interval, filter.as_deref())
        }
    };

    if let Err(e) = result {
        eprintln!("ERROR: {}", e);
        std::process::exit(1);
    }
}
