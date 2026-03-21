use std::fs;
use std::io::{self, BufRead, Read};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::snapshot;

const PERF_STATE_DIR: &str = "/tmp/agent-terminal-perf";

#[derive(Serialize)]
struct FrameEvent {
    t_ms: u64,
    frame_ms: u64,
}

#[derive(Serialize)]
struct FpsResult {
    fps: f64,
    frame_count: u64,
    duration_ms: u64,
    min_frame_ms: u64,
    max_frame_ms: u64,
    mean_frame_ms: f64,
    p95_frame_ms: u64,
    idle_ms: u64,
    timeline: Vec<FrameEvent>,
}

#[derive(Serialize)]
struct LatencyResult {
    mean_ms: f64,
    min_ms: u64,
    max_ms: u64,
    p95_ms: u64,
    samples: u32,
    measurements: Vec<u64>,
}

fn perf_pid_file(session: &str) -> String {
    format!("{}/{}-pid", PERF_STATE_DIR, session)
}

fn perf_data_file(session: &str) -> String {
    format!("{}/{}-frames.jsonl", PERF_STATE_DIR, session)
}

/// Start background frame recording for a session.
/// Spawns a background process that polls capture-pane and records frame changes.
pub fn start(session: &str) -> Result<(), String> {
    fs::create_dir_all(PERF_STATE_DIR).map_err(|e| format!("Failed to create perf dir: {}", e))?;

    let pid_file = perf_pid_file(session);
    let data_file = perf_data_file(session);

    // Clean up any existing data
    let _ = fs::remove_file(&data_file);
    let _ = fs::remove_file(&pid_file);

    // Build the polling script that runs as a background process
    // It polls tmux capture-pane and records when content changes
    let script = format!(
        r#"#!/bin/sh
DATA_FILE="{data_file}"
SESSION="{session}"
echo $$ > "{pid_file}"
LAST_CONTENT=""
START_MS=$(python3 -c 'import time; print(int(time.time()*1000))' 2>/dev/null || date +%s000)
LAST_CHANGE_MS=$START_MS

while true; do
    CONTENT=$(tmux capture-pane -t "$SESSION" -p 2>/dev/null)
    NOW_MS=$(python3 -c 'import time; print(int(time.time()*1000))' 2>/dev/null || date +%s000)
    if [ "$CONTENT" != "$LAST_CONTENT" ] && [ -n "$LAST_CONTENT" ]; then
        FRAME_MS=$((NOW_MS - LAST_CHANGE_MS))
        T_MS=$((NOW_MS - START_MS))
        echo "{{"t_ms\":$T_MS,\"frame_ms\":$FRAME_MS}}" >> "$DATA_FILE"
        LAST_CHANGE_MS=$NOW_MS
    fi
    if [ -z "$LAST_CONTENT" ]; then
        LAST_CONTENT="$CONTENT"
        LAST_CHANGE_MS=$NOW_MS
    else
        LAST_CONTENT="$CONTENT"
    fi
    # Poll every 10ms
    sleep 0.01
done
"#
    );

    let script_file = format!("{}/{}-poller.sh", PERF_STATE_DIR, session);
    fs::write(&script_file, &script).map_err(|e| format!("Failed to write poller script: {}", e))?;

    // Launch the background poller
    Command::new("sh")
        .arg(&script_file)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start perf poller: {}", e))?;

    // Wait briefly for the PID file to appear
    for _ in 0..20 {
        if fs::metadata(&pid_file).is_ok() {
            println!("Perf recording started for session '{}'", session);
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }

    println!("Perf recording started for session '{}'", session);
    Ok(())
}

/// Stop frame recording and return metrics.
pub fn stop(json: bool, session: &str) -> Result<(), String> {
    let pid_file = perf_pid_file(session);
    let data_file = perf_data_file(session);

    // Kill the poller process
    if let Ok(pid_str) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }
    }

    // Read frame data
    let frames = read_frame_data(&data_file);

    // Clean up
    let _ = fs::remove_file(&pid_file);
    let _ = fs::remove_file(&data_file);
    let script_file = format!("{}/{}-poller.sh", PERF_STATE_DIR, session);
    let _ = fs::remove_file(&script_file);

    if frames.is_empty() {
        let result = FpsResult {
            fps: 0.0,
            frame_count: 0,
            duration_ms: 0,
            min_frame_ms: 0,
            max_frame_ms: 0,
            mean_frame_ms: 0.0,
            p95_frame_ms: 0,
            idle_ms: 0,
            timeline: vec![],
        };
        output_fps_result(&result, json);
        return Ok(());
    }

    let result = compute_fps_metrics(&frames);
    output_fps_result(&result, json);
    Ok(())
}

/// Measure FPS during a command or for a duration.
pub fn fps(during: Option<&str>, during_batch: bool, duration: Option<u64>, session: &str) -> Result<(), String> {
    if let Some(dur_ms) = duration {
        // Passive observation for N ms
        let frames = record_frames_for(session, dur_ms)?;
        let result = compute_fps_metrics(&frames);
        output_fps_result(&result, true);
        return Ok(());
    }

    if let Some(cmd_str) = during {
        // Start recording, run commands, stop recording
        let start_time = Instant::now();
        let mut frames = Vec::new();
        let mut last_content = snapshot::capture_plain(session, None).unwrap_or_default();

        // Parse and execute commands (simple: split by &&)
        let commands: Vec<&str> = cmd_str.split("&&").map(|s| s.trim()).collect();

        for cmd in &commands {
            // Execute each agent-terminal command
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // Build the command - prepend agent-terminal and session
            let exe = std::env::current_exe().map_err(|e| format!("Cannot find self: {}", e))?;
            let mut c = Command::new(&exe);
            c.args(parts);
            c.arg("--session").arg(session);
            let _ = c.output();

            // Check for frame change
            if let Ok(content) = snapshot::capture_plain(session, None) {
                if content != last_content {
                    let elapsed = start_time.elapsed().as_millis() as u64;
                    frames.push(FrameEvent {
                        t_ms: elapsed,
                        frame_ms: elapsed,
                    });
                    last_content = content;
                }
            }
        }

        // Continue recording briefly after commands
        let poll_end = Instant::now() + Duration::from_millis(500);
        while Instant::now() < poll_end {
            if let Ok(content) = snapshot::capture_plain(session, None) {
                if content != last_content {
                    let elapsed = start_time.elapsed().as_millis() as u64;
                    let frame_ms = frames.last().map(|f| elapsed - f.t_ms).unwrap_or(elapsed);
                    frames.push(FrameEvent { t_ms: elapsed, frame_ms });
                    last_content = content;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        let result = compute_fps_metrics(&frames);
        output_fps_result(&result, true);
        return Ok(());
    }

    if during_batch {
        // Read JSON batch from stdin
        let mut input = String::new();
        io::stdin().read_to_string(&mut input).map_err(|e| format!("Failed to read stdin: {}", e))?;

        let batch: Vec<serde_json::Value> = serde_json::from_str(&input)
            .map_err(|e| format!("Invalid JSON batch: {}", e))?;

        let start_time = Instant::now();
        let mut frames = Vec::new();
        let mut last_content = snapshot::capture_plain(session, None).unwrap_or_default();
        let exe = std::env::current_exe().map_err(|e| format!("Cannot find self: {}", e))?;

        for item in &batch {
            let cmd = item["cmd"].as_str().unwrap_or("");
            let args: Vec<&str> = item["args"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            let mut c = Command::new(&exe);
            c.arg(cmd);
            c.args(&args);
            c.arg("--session").arg(session);
            let _ = c.output();

            if let Ok(content) = snapshot::capture_plain(session, None) {
                if content != last_content {
                    let elapsed = start_time.elapsed().as_millis() as u64;
                    let frame_ms = frames.last().map(|f: &FrameEvent| elapsed - f.t_ms).unwrap_or(elapsed);
                    frames.push(FrameEvent { t_ms: elapsed, frame_ms });
                    last_content = content;
                }
            }
        }

        let result = compute_fps_metrics(&frames);
        output_fps_result(&result, true);
        return Ok(());
    }

    Err("perf fps requires --during, --during-batch, or --duration".to_string())
}

/// Measure input latency.
pub fn latency(key: Option<&str>, samples: u32, json: bool, session: &str) -> Result<(), String> {
    let test_key = key.unwrap_or("space");
    let cancel_key = if test_key == "space" { Some("BSpace") } else { None };

    let mut measurements = Vec::new();

    for _ in 0..samples {
        // Capture current state
        let before = snapshot::capture_plain(session, None)
            .map_err(|e| format!("Failed to capture before: {}", e))?;

        // Send the key
        let target = if let Some(_) = None::<&str> {
            session.to_string()
        } else {
            session.to_string()
        };

        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &target, test_key])
            .output()
            .map_err(|e| format!("Failed to send key: {}", e))?;

        // Poll until content changes
        let start = Instant::now();
        let timeout = Duration::from_millis(5000);
        let mut latency_ms = None;

        while start.elapsed() < timeout {
            if let Ok(after) = snapshot::capture_plain(session, None) {
                if after != before {
                    latency_ms = Some(start.elapsed().as_millis() as u64);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(1));
        }

        if let Some(ms) = latency_ms {
            measurements.push(ms);
        }

        // Send cancel key if needed to avoid side effects
        if let Some(ck) = cancel_key {
            let _ = Command::new("tmux")
                .args(["send-keys", "-t", &target, ck])
                .output();
            thread::sleep(Duration::from_millis(50));
        }

        // Brief pause between samples
        thread::sleep(Duration::from_millis(100));
    }

    if measurements.is_empty() {
        return Err("No latency measurements could be taken (screen never changed)".to_string());
    }

    let mut sorted = measurements.clone();
    sorted.sort();

    let sum: u64 = sorted.iter().sum();
    let mean_ms = sum as f64 / sorted.len() as f64;
    let min_ms = sorted[0];
    let max_ms = sorted[sorted.len() - 1];
    let p95_idx = ((sorted.len() as f64 * 0.95) as usize).min(sorted.len() - 1);
    let p95_ms = sorted[p95_idx];

    let result = LatencyResult {
        mean_ms,
        min_ms,
        max_ms,
        p95_ms,
        samples: sorted.len() as u32,
        measurements: sorted,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        println!("Input latency ({} samples):", result.samples);
        println!("  mean: {:.1}ms", result.mean_ms);
        println!("  min:  {}ms", result.min_ms);
        println!("  max:  {}ms", result.max_ms);
        println!("  p95:  {}ms", result.p95_ms);

        // Interpretation
        if result.mean_ms < 16.0 {
            println!("  Rating: excellent (imperceptible)");
        } else if result.mean_ms < 50.0 {
            println!("  Rating: good (responsive)");
        } else if result.mean_ms < 100.0 {
            println!("  Rating: fair (noticeable lag)");
        } else if result.mean_ms < 200.0 {
            println!("  Rating: poor (sluggish)");
        } else {
            println!("  Rating: bad (likely blocking render loop)");
        }
    }

    Ok(())
}

// --- Internal helpers ---

fn read_frame_data(path: &str) -> Vec<FrameEvent> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let reader = io::BufReader::new(file);
    let mut frames = Vec::new();

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                let t_ms = event["t_ms"].as_u64().unwrap_or(0);
                let frame_ms = event["frame_ms"].as_u64().unwrap_or(0);
                frames.push(FrameEvent { t_ms, frame_ms });
            }
        }
    }

    frames
}

fn record_frames_for(session: &str, duration_ms: u64) -> Result<Vec<FrameEvent>, String> {
    let start = Instant::now();
    let duration = Duration::from_millis(duration_ms);
    let mut frames = Vec::new();
    let mut last_content = snapshot::capture_plain(session, None).unwrap_or_default();

    while start.elapsed() < duration {
        if let Ok(content) = snapshot::capture_plain(session, None) {
            if content != last_content {
                let elapsed = start.elapsed().as_millis() as u64;
                let frame_ms = frames.last().map(|f: &FrameEvent| elapsed - f.t_ms).unwrap_or(elapsed);
                frames.push(FrameEvent { t_ms: elapsed, frame_ms });
                last_content = content;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    Ok(frames)
}

fn compute_fps_metrics(frames: &[FrameEvent]) -> FpsResult {
    if frames.is_empty() {
        return FpsResult {
            fps: 0.0,
            frame_count: 0,
            duration_ms: 0,
            min_frame_ms: 0,
            max_frame_ms: 0,
            mean_frame_ms: 0.0,
            p95_frame_ms: 0,
            idle_ms: 0,
            timeline: vec![],
        };
    }

    let frame_count = frames.len() as u64;
    let duration_ms = frames.last().map(|f| f.t_ms).unwrap_or(0);

    let mut frame_times: Vec<u64> = frames.iter().map(|f| f.frame_ms).collect();
    frame_times.sort();

    let min_frame_ms = frame_times[0];
    let max_frame_ms = frame_times[frame_times.len() - 1];
    let sum: u64 = frame_times.iter().sum();
    let mean_frame_ms = sum as f64 / frame_times.len() as f64;
    let p95_idx = ((frame_times.len() as f64 * 0.95) as usize).min(frame_times.len() - 1);
    let p95_frame_ms = frame_times[p95_idx];

    let fps = if duration_ms > 0 {
        (frame_count as f64 / duration_ms as f64) * 1000.0
    } else {
        0.0
    };

    // Idle time = total duration - sum of frame times
    let idle_ms = if duration_ms > sum { duration_ms - sum } else { 0 };

    let timeline: Vec<FrameEvent> = frames
        .iter()
        .map(|f| FrameEvent {
            t_ms: f.t_ms,
            frame_ms: f.frame_ms,
        })
        .collect();

    FpsResult {
        fps,
        frame_count,
        duration_ms,
        min_frame_ms,
        max_frame_ms,
        mean_frame_ms,
        p95_frame_ms,
        idle_ms,
        timeline,
    }
}

fn output_fps_result(result: &FpsResult, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(result).unwrap());
    } else {
        println!("Performance metrics:");
        println!("  FPS: {:.1}", result.fps);
        println!("  Frames: {}", result.frame_count);
        println!("  Duration: {}ms", result.duration_ms);
        println!("  Frame time: min={}ms mean={:.1}ms max={}ms p95={}ms",
                 result.min_frame_ms, result.mean_frame_ms, result.max_frame_ms, result.p95_frame_ms);
        println!("  Idle: {}ms", result.idle_ms);

        // Interpretation
        if result.fps == 0.0 {
            println!("  Rating: no frames detected (app may be frozen)");
        } else if result.fps < 5.0 {
            println!("  Rating: sluggish (likely blocking)");
        } else if result.fps < 10.0 {
            println!("  Rating: acceptable");
        } else {
            println!("  Rating: good");
        }
    }
}
