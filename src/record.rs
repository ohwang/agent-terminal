use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::snapshot;

const DEFAULT_RECORDINGS_DIR: &str = ".agent-terminal/recordings";
const RECORDING_STATE_PREFIX: &str = "/tmp/agent-terminal-recording-";
const DEFAULT_FPS: u32 = 10;

#[derive(Serialize, Deserialize)]
struct RecordingMeta {
    session: String,
    group: String,
    label: String,
    started_at: String,
    stopped_at: Option<String>,
    cols: u16,
    rows: u16,
    frame_count: u64,
    duration_ms: u64,
}

#[derive(Serialize, Deserialize)]
struct FrameEntry {
    timestamp_ms: f64,
    text: String,
    cols: u16,
    rows: u16,
    cursor_row: u16,
    cursor_col: u16,
}

#[derive(Serialize, Deserialize)]
pub struct ActionEntry {
    pub timestamp_ms: f64,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Serialize)]
struct CastHeader {
    version: u32,
    width: u16,
    height: u16,
    timestamp: i64,
}

fn default_recordings_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    Ok(home.join(DEFAULT_RECORDINGS_DIR))
}

fn recordings_dir(dir: Option<&str>) -> Result<PathBuf, String> {
    match dir {
        Some(d) => Ok(PathBuf::from(d)),
        None => default_recordings_dir(),
    }
}

fn state_marker_path(session: &str) -> String {
    format!("{}{}", RECORDING_STATE_PREFIX, session)
}

fn recording_dir_name(session: &str, label: &str) -> String {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    if label.is_empty() {
        format!("{}_{}", ts, session)
    } else {
        format!("{}_{}_{}", ts, session, label)
    }
}

/// Start recording a session. Spawns a background poller process.
pub fn start(
    session: &str,
    group: &str,
    label: &str,
    fps: Option<u32>,
    dir: Option<&str>,
) -> Result<(), String> {
    // Check if already recording this session
    let marker = state_marker_path(session);
    if Path::new(&marker).exists() {
        // Check if the poller is still alive
        let existing_dir = fs::read_to_string(&marker).unwrap_or_default();
        let pid_file = Path::new(existing_dir.trim()).join("pid");
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                if nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid),
                    nix::sys::signal::Signal::SIGCONT,
                )
                .is_ok()
                {
                    return Err(format!(
                        "Session '{}' is already being recorded. Stop it first with: agent-terminal record stop --session {}",
                        session, session
                    ));
                }
            }
        }
        // Stale marker, clean up
        let _ = fs::remove_file(&marker);
    }

    // Get terminal size
    let (cols, rows, _, _) = snapshot::get_pane_info(session, None)?;

    // Create recording directory
    let base = recordings_dir(dir)?;
    let group_dir = base.join(group);
    let rec_name = recording_dir_name(session, label);
    let rec_dir = group_dir.join(&rec_name);
    fs::create_dir_all(&rec_dir)
        .map_err(|e| format!("Failed to create recording directory: {}", e))?;

    // Write initial meta.json
    let meta = RecordingMeta {
        session: session.to_string(),
        group: group.to_string(),
        label: label.to_string(),
        started_at: chrono::Local::now().to_rfc3339(),
        stopped_at: None,
        cols,
        rows,
        frame_count: 0,
        duration_ms: 0,
    };
    let meta_path = rec_dir.join("meta.json");
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Failed to serialize meta.json: {}", e))?;
    fs::write(&meta_path, meta_json)
        .map_err(|e| format!("Failed to write meta.json: {}", e))?;

    // Create empty actions.jsonl
    fs::write(rec_dir.join("actions.jsonl"), "")
        .map_err(|e| format!("Failed to create actions.jsonl: {}", e))?;

    // Write state marker (recording dir path)
    fs::write(&marker, rec_dir.to_string_lossy().as_ref())
        .map_err(|e| format!("Failed to write state marker: {}", e))?;

    let fps = fps.unwrap_or(DEFAULT_FPS);

    // Spawn background poller via re-invoking ourselves
    let exe = std::env::current_exe()
        .map_err(|e| format!("Failed to find current executable: {}", e))?;

    let child = Command::new(&exe)
        .args([
            "record",
            "__poll",
            "--session",
            session,
            "--recording-dir",
            &rec_dir.to_string_lossy(),
            "--fps",
            &fps.to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start recording poller: {}", e))?;

    // Write PID file
    let pid_file = rec_dir.join("pid");
    fs::write(&pid_file, child.id().to_string())
        .map_err(|e| format!("Failed to write PID file: {}", e))?;

    // Wait briefly for the poller to start capturing
    for _ in 0..20 {
        if rec_dir.join("recording.cast").exists() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    println!(
        "Recording started for session '{}' (group: {}, label: {}, fps: {})",
        session,
        group,
        if label.is_empty() { "<none>" } else { label },
        fps
    );
    println!("Recording dir: {}", rec_dir.display());

    Ok(())
}

/// Stop recording a session.
pub fn stop(session: &str) -> Result<(), String> {
    let marker = state_marker_path(session);
    let rec_dir_str = fs::read_to_string(&marker).map_err(|_| {
        format!(
            "No active recording found for session '{}'. Is recording started?",
            session
        )
    })?;
    let rec_dir = PathBuf::from(rec_dir_str.trim());

    // Kill the poller
    let pid_file = rec_dir.join("pid");
    if let Ok(pid_str) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid),
                nix::sys::signal::Signal::SIGTERM,
            );
            // Wait briefly for it to exit
            for _ in 0..20 {
                if nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid),
                    nix::sys::signal::Signal::SIGCONT,
                )
                .is_err()
                {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
    let _ = fs::remove_file(&pid_file);

    // Update meta.json with final stats
    let meta_path = rec_dir.join("meta.json");
    if let Ok(meta_str) = fs::read_to_string(&meta_path) {
        if let Ok(mut meta) = serde_json::from_str::<RecordingMeta>(&meta_str) {
            meta.stopped_at = Some(chrono::Local::now().to_rfc3339());

            // Count frames from frames.jsonl
            let frames_path = rec_dir.join("frames.jsonl");
            if let Ok(frames_content) = fs::read_to_string(&frames_path) {
                meta.frame_count = frames_content.lines().filter(|l| !l.is_empty()).count() as u64;
            }

            // Calculate duration from started_at to now
            if let Ok(started) = chrono::DateTime::parse_from_rfc3339(&meta.started_at) {
                let duration = chrono::Local::now().signed_duration_since(started);
                meta.duration_ms = duration.num_milliseconds().max(0) as u64;
            }

            if let Ok(json) = serde_json::to_string_pretty(&meta) {
                let _ = fs::write(&meta_path, json);
            }

            println!(
                "Recording stopped for session '{}': {} frames, {}ms",
                session, meta.frame_count, meta.duration_ms
            );
        }
    }

    println!("Recording dir: {}", rec_dir.display());

    // Remove state marker
    let _ = fs::remove_file(&marker);

    Ok(())
}

/// Background poller — hidden subcommand entry point.
/// Captures frames at the configured FPS, writing .cast and frames.jsonl.
pub fn poll(session: &str, recording_dir: &str, fps: u32) -> Result<(), String> {
    let rec_dir = PathBuf::from(recording_dir);
    let cast_path = rec_dir.join("recording.cast");
    let frames_path = rec_dir.join("frames.jsonl");

    let interval = Duration::from_millis((1000 / fps).max(10) as u64);

    // Get initial terminal info
    let (cols, rows, _, _) = snapshot::get_pane_info(session, None)?;

    // Write .cast header
    let header = CastHeader {
        version: 2,
        width: cols,
        height: rows,
        timestamp: chrono::Utc::now().timestamp(),
    };
    let mut cast_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&cast_path)
        .map_err(|e| format!("Failed to create cast file: {}", e))?;
    let header_json = serde_json::to_string(&header)
        .map_err(|e| format!("Failed to serialize cast header: {}", e))?;
    writeln!(cast_file, "{}", header_json)
        .map_err(|e| format!("Failed to write cast header: {}", e))?;

    let mut frames_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&frames_path)
        .map_err(|e| format!("Failed to create frames file: {}", e))?;

    // Set up SIGTERM handler for graceful shutdown
    SHOULD_STOP.store(false, Ordering::SeqCst);
    let _ = unsafe {
        nix::sys::signal::signal(
            nix::sys::signal::Signal::SIGTERM,
            nix::sys::signal::SigHandler::Handler(handle_sigterm),
        )
    };

    let start_time = Instant::now();
    let mut last_plain = String::new();

    while !SHOULD_STOP.load(Ordering::SeqCst) {
        let tick_start = Instant::now();

        // Capture current state
        let plain = match snapshot::capture_plain(session, None) {
            Ok(p) => p,
            Err(_) => {
                // Session may have been closed — exit gracefully
                break;
            }
        };

        // Only record if content changed
        if plain != last_plain {
            let ansi = match snapshot::capture_ansi(session, None) {
                Ok(a) => a,
                Err(_) => break,
            };

            let (cur_cols, cur_rows, cursor_x, cursor_y) =
                snapshot::get_pane_info(session, None).unwrap_or((cols, rows, 0, 0));

            let elapsed = start_time.elapsed().as_secs_f64();

            // Write .cast event: clear screen + home + full ANSI content
            let cast_data = format!("\x1b[2J\x1b[H{}", ansi);
            let cast_event = serde_json::json!([elapsed, "o", cast_data]);
            let _ = writeln!(cast_file, "{}", cast_event);

            // Write frames.jsonl entry
            let frame = FrameEntry {
                timestamp_ms: elapsed * 1000.0,
                text: plain.clone(),
                cols: cur_cols,
                rows: cur_rows,
                cursor_row: cursor_y,
                cursor_col: cursor_x,
            };
            if let Ok(json) = serde_json::to_string(&frame) {
                let _ = writeln!(frames_file, "{}", json);
            }

            last_plain = plain;
        }

        // Sleep for remaining interval time
        let elapsed = tick_start.elapsed();
        if elapsed < interval {
            thread::sleep(interval - elapsed);
        }
    }

    Ok(())
}

// Static atomic for signal handler
static SHOULD_STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigterm(_: i32) {
    SHOULD_STOP.store(true, Ordering::SeqCst);
}

/// List all recordings.
pub fn list(dir: Option<&str>, json: bool) -> Result<(), String> {
    let base = recordings_dir(dir)?;

    if !base.exists() {
        if json {
            println!("[]");
        } else {
            println!("No recordings found.");
        }
        return Ok(());
    }

    let mut recordings: Vec<RecordingMeta> = Vec::new();

    // Walk group directories
    let entries = fs::read_dir(&base).map_err(|e| format!("Failed to read recordings dir: {}", e))?;
    for group_entry in entries.flatten() {
        if !group_entry.path().is_dir() {
            continue;
        }
        let rec_entries =
            fs::read_dir(group_entry.path()).map_err(|e| format!("Failed to read group dir: {}", e))?;
        for rec_entry in rec_entries.flatten() {
            let meta_path = rec_entry.path().join("meta.json");
            if let Ok(meta_str) = fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<RecordingMeta>(&meta_str) {
                    recordings.push(meta);
                }
            }
        }
    }

    // Sort by started_at descending
    recordings.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    if json {
        let json = serde_json::to_string_pretty(&recordings)
            .map_err(|e| format!("Failed to serialize recordings: {}", e))?;
        println!("{}", json);
        return Ok(());
    }

    if recordings.is_empty() {
        println!("No recordings found.");
        return Ok(());
    }

    // Group by group name
    let mut groups: std::collections::BTreeMap<String, Vec<&RecordingMeta>> =
        std::collections::BTreeMap::new();
    for rec in &recordings {
        groups.entry(rec.group.clone()).or_default().push(rec);
    }

    for (group, recs) in &groups {
        println!("Group: {}", group);
        for rec in recs {
            let label_str = if rec.label.is_empty() {
                String::new()
            } else {
                format!(" [{}]", rec.label)
            };
            let status = if rec.stopped_at.is_some() {
                format!("{} frames, {}ms", rec.frame_count, rec.duration_ms)
            } else {
                "recording...".to_string()
            };
            println!(
                "  {} session={}{} ({}x{}) {}",
                rec.started_at, rec.session, label_str, rec.cols, rec.rows, status
            );
        }
    }

    Ok(())
}

/// View a recording as a chronological text stream.
/// Default mode shows key frames (before/after each action + final frame).
/// --all-frames shows every frame interleaved with actions.
pub fn view(dir: &str, all_frames: bool, json: bool) -> Result<(), String> {
    let rec_dir = PathBuf::from(dir);
    if !rec_dir.exists() {
        return Err(format!("Recording directory not found: {}", dir));
    }

    // Read metadata
    let meta_path = rec_dir.join("meta.json");
    let meta: Option<RecordingMeta> = fs::read_to_string(&meta_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());

    // Read frames
    let frames_path = rec_dir.join("frames.jsonl");
    let frames: Vec<FrameEntry> = fs::read_to_string(&frames_path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    // Read actions
    let actions_path = rec_dir.join("actions.jsonl");
    let actions: Vec<ActionEntry> = fs::read_to_string(&actions_path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    // Select which frame indices to include
    let key_frame_indices: std::collections::BTreeSet<usize> = if all_frames {
        (0..frames.len()).collect()
    } else {
        select_key_frames(&frames, &actions)
    };

    // Build unified timeline sorted by timestamp
    #[derive(PartialEq)]
    enum EventKind {
        Frame(usize),
        Action(usize),
    }
    let mut timeline: Vec<(f64, EventKind)> = Vec::new();
    for (i, f) in frames.iter().enumerate() {
        if key_frame_indices.contains(&i) {
            timeline.push((f.timestamp_ms, EventKind::Frame(i)));
        }
    }
    for (i, a) in actions.iter().enumerate() {
        timeline.push((a.timestamp_ms, EventKind::Action(i)));
    }
    // Stable sort: preserve frame-before-action ordering for same timestamp
    timeline.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if json {
        // JSON output
        let mut events: Vec<serde_json::Value> = Vec::new();
        for (_, kind) in &timeline {
            match kind {
                EventKind::Frame(i) => {
                    let f = &frames[*i];
                    events.push(serde_json::json!({
                        "type": "frame",
                        "timestamp_ms": f.timestamp_ms,
                        "text": f.text,
                        "cols": f.cols,
                        "rows": f.rows,
                        "cursor_row": f.cursor_row,
                        "cursor_col": f.cursor_col,
                    }));
                }
                EventKind::Action(i) => {
                    let a = &actions[*i];
                    events.push(serde_json::json!({
                        "type": "action",
                        "timestamp_ms": a.timestamp_ms,
                        "command": a.command,
                        "args": a.args,
                    }));
                }
            }
        }
        let json = serde_json::to_string_pretty(&events)
            .map_err(|e| format!("Failed to serialize events: {}", e))?;
        println!("{}", json);
    } else {
        // Text output
        if let Some(ref m) = meta {
            println!(
                "=== Recording: session={}, group={}, label={} ===",
                m.session,
                m.group,
                if m.label.is_empty() { "<none>" } else { &m.label }
            );
            println!(
                "=== {}x{}, {}ms, {} frames, {} actions ===",
                m.cols,
                m.rows,
                m.duration_ms,
                frames.len(),
                actions.len()
            );
            if !all_frames {
                println!(
                    "=== Showing {} key frames (use --all-frames for full timeline) ===",
                    key_frame_indices.len()
                );
            }
            println!();
        }

        for (_, kind) in &timeline {
            match kind {
                EventKind::Frame(i) => {
                    let f = &frames[*i];
                    println!("--- Frame @ {}ms ({}x{}) ---", f.timestamp_ms as u64, f.cols, f.rows);
                    println!("{}", f.text);
                    println!();
                }
                EventKind::Action(i) => {
                    let a = &actions[*i];
                    println!(
                        ">>> Action @ {}ms: {} {:?}",
                        a.timestamp_ms as u64, a.command, a.args
                    );
                    println!();
                }
            }
        }
    }

    Ok(())
}

fn select_key_frames(
    frames: &[FrameEntry],
    actions: &[ActionEntry],
) -> std::collections::BTreeSet<usize> {
    let mut key_indices = std::collections::BTreeSet::new();

    // Always include the final frame
    if !frames.is_empty() {
        key_indices.insert(frames.len() - 1);
    }

    // If no actions, also include the first frame
    if actions.is_empty() {
        if !frames.is_empty() {
            key_indices.insert(0);
        }
        return key_indices;
    }

    for action in actions {
        let t = action.timestamp_ms;

        // Frame immediately before (or at) this action:
        // partition_point returns the first index where timestamp_ms > t
        let before_idx = frames.partition_point(|f| f.timestamp_ms <= t);
        if before_idx > 0 {
            key_indices.insert(before_idx - 1);
        }

        // Frame immediately after this action
        if before_idx < frames.len() {
            key_indices.insert(before_idx);
        }
    }

    key_indices
}

/// Log an action to the active recording for a session.
/// This is best-effort — it never returns an error and silently does nothing
/// if no recording is active.
pub fn log_action(session: &str, command: &str, args: &[String]) {
    let marker = state_marker_path(session);

    // Fast path: check if marker exists
    if fs::metadata(&marker).is_err() {
        return;
    }

    let rec_dir_str = match fs::read_to_string(&marker) {
        Ok(s) => s,
        Err(_) => return,
    };
    let rec_dir = PathBuf::from(rec_dir_str.trim());

    // Read started_at from meta.json to compute relative timestamp
    let meta_path = rec_dir.join("meta.json");
    let started_at = match fs::read_to_string(&meta_path) {
        Ok(meta_str) => match serde_json::from_str::<RecordingMeta>(&meta_str) {
            Ok(meta) => meta.started_at,
            Err(_) => return,
        },
        Err(_) => return,
    };

    let timestamp_ms = match chrono::DateTime::parse_from_rfc3339(&started_at) {
        Ok(start) => {
            let elapsed = chrono::Local::now().signed_duration_since(start);
            elapsed.num_milliseconds().max(0) as f64
        }
        Err(_) => return,
    };

    let entry = ActionEntry {
        timestamp_ms,
        command: command.to_string(),
        args: args.to_vec(),
    };

    let actions_path = rec_dir.join("actions.jsonl");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&actions_path) {
        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = writeln!(file, "{}", json);
        }
    }
}
