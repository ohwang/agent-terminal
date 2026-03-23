mod common;
use common::Session;
use std::process::Command;

fn bin() -> String {
    Session::bin_path()
}

/// Run a raw agent-terminal command (no auto session injection).
fn run_raw(args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("failed to run agent-terminal")
}

fn run_raw_ok(args: &[&str]) -> String {
    let out = run_raw(args);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        out.status.success(),
        "Command {:?} failed (exit {})\nstdout: {}\nstderr: {}",
        args,
        out.status.code().unwrap_or(-1),
        stdout,
        stderr
    );
    stdout
}

#[test]
fn test_record_start_stop() {
    let s = Session::new();
    let rec_dir = tempfile::tempdir().unwrap();
    let rec_dir_str = rec_dir.path().to_string_lossy().to_string();

    // Open a fixture
    s.open_fixture_wait("counter", "Count: 0");

    // Start recording
    run_raw_ok(&[
        "record", "start",
        "--session", &s.name,
        "--group", "test-group",
        "--label", "basic",
        "--dir", &rec_dir_str,
    ]);

    // Wait a bit for some frames to be captured
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Interact to generate a frame change
    s.run_ok(&["send", "j"]);
    s.run_ok(&["wait", "--text", "Count: 1", "--timeout", "3000"]);

    std::thread::sleep(std::time::Duration::from_millis(500));

    // Stop recording
    run_raw_ok(&[
        "record", "stop",
        "--session", &s.name,
    ]);

    // Find the recording directory
    let group_dir = rec_dir.path().join("test-group");
    assert!(group_dir.exists(), "Group directory should exist");

    let entries: Vec<_> = std::fs::read_dir(&group_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1, "Should have exactly one recording");

    let recording_dir = entries[0].path();

    // Check all files exist
    assert!(recording_dir.join("meta.json").exists(), "meta.json should exist");
    assert!(recording_dir.join("recording.cast").exists(), "recording.cast should exist");
    assert!(recording_dir.join("frames.jsonl").exists(), "frames.jsonl should exist");
    assert!(recording_dir.join("actions.jsonl").exists(), "actions.jsonl should exist");
    assert!(!recording_dir.join("pid").exists(), "pid file should be cleaned up");

    // Validate meta.json
    let meta_str = std::fs::read_to_string(recording_dir.join("meta.json")).unwrap();
    let meta: serde_json::Value = serde_json::from_str(&meta_str).unwrap();
    assert_eq!(meta["session"].as_str().unwrap(), &s.name);
    assert_eq!(meta["group"].as_str().unwrap(), "test-group");
    assert_eq!(meta["label"].as_str().unwrap(), "basic");
    assert!(meta["stopped_at"].as_str().is_some(), "Should have stopped_at");
    assert!(meta["frame_count"].as_u64().unwrap() >= 1, "Should have at least 1 frame");
    assert!(meta["duration_ms"].as_u64().unwrap() > 0, "Duration should be positive");

    // Validate .cast file
    let cast_str = std::fs::read_to_string(recording_dir.join("recording.cast")).unwrap();
    let cast_lines: Vec<&str> = cast_str.lines().collect();
    assert!(cast_lines.len() >= 2, "Cast file should have header + at least 1 event");

    // Header should be valid JSON with version 2
    let header: serde_json::Value = serde_json::from_str(cast_lines[0]).unwrap();
    assert_eq!(header["version"].as_u64().unwrap(), 2);

    // Events should be arrays [time, "o", data]
    let event: serde_json::Value = serde_json::from_str(cast_lines[1]).unwrap();
    assert!(event.is_array());
    assert_eq!(event[1].as_str().unwrap(), "o");

    // Validate frames.jsonl
    let frames_str = std::fs::read_to_string(recording_dir.join("frames.jsonl")).unwrap();
    let frame_lines: Vec<&str> = frames_str.lines().filter(|l| !l.is_empty()).collect();
    assert!(!frame_lines.is_empty(), "Should have at least one frame");

    let frame: serde_json::Value = serde_json::from_str(frame_lines[0]).unwrap();
    assert!(frame["timestamp_ms"].is_number());
    assert!(frame["text"].is_string());
    assert!(frame["cols"].is_number());
    assert!(frame["rows"].is_number());
}

#[test]
fn test_record_action_logging() {
    let s = Session::new();
    let rec_dir = tempfile::tempdir().unwrap();
    let rec_dir_str = rec_dir.path().to_string_lossy().to_string();

    s.open_fixture_wait("counter", "Count: 0");

    // Start recording
    run_raw_ok(&[
        "record", "start",
        "--session", &s.name,
        "--group", "action-test",
        "--label", "log",
        "--dir", &rec_dir_str,
    ]);

    std::thread::sleep(std::time::Duration::from_millis(300));

    // Perform some actions
    s.run_ok(&["send", "j"]);
    s.run_ok(&["wait", "--text", "Count: 1", "--timeout", "3000"]);
    s.run_ok(&["send", "j"]);

    std::thread::sleep(std::time::Duration::from_millis(300));

    // Stop recording
    run_raw_ok(&[
        "record", "stop",
        "--session", &s.name,
    ]);

    // Find the recording
    let group_dir = rec_dir.path().join("action-test");
    let entries: Vec<_> = std::fs::read_dir(&group_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    let recording_dir = entries[0].path();

    // Check actions.jsonl
    let actions_str = std::fs::read_to_string(recording_dir.join("actions.jsonl")).unwrap();
    let action_lines: Vec<&str> = actions_str.lines().filter(|l| !l.is_empty()).collect();

    // Should have at least the send and wait actions
    assert!(
        action_lines.len() >= 2,
        "Should have at least 2 actions logged, got {}",
        action_lines.len()
    );

    // Validate first action is a send
    let first: serde_json::Value = serde_json::from_str(action_lines[0]).unwrap();
    assert_eq!(first["command"].as_str().unwrap(), "send");
    assert!(first["timestamp_ms"].as_f64().unwrap() >= 0.0);
}

#[test]
fn test_record_deduplication() {
    let s = Session::new();
    let rec_dir = tempfile::tempdir().unwrap();
    let rec_dir_str = rec_dir.path().to_string_lossy().to_string();

    // Open fixture and wait for it to stabilize
    s.open_fixture_wait("counter", "Count: 0");

    // Start recording — don't interact, so screen should stay the same
    run_raw_ok(&[
        "record", "start",
        "--session", &s.name,
        "--group", "dedup-test",
        "--dir", &rec_dir_str,
    ]);

    // Wait 2 seconds without any interaction
    std::thread::sleep(std::time::Duration::from_millis(2000));

    // Stop recording
    run_raw_ok(&[
        "record", "stop",
        "--session", &s.name,
    ]);

    // Find the recording
    let group_dir = rec_dir.path().join("dedup-test");
    let entries: Vec<_> = std::fs::read_dir(&group_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    let recording_dir = entries[0].path();

    // Should have very few frames (ideally just 1 — the initial capture)
    let frames_str = std::fs::read_to_string(recording_dir.join("frames.jsonl")).unwrap();
    let frame_count = frames_str.lines().filter(|l| !l.is_empty()).count();

    // With deduplication, a static screen should produce very few frames
    // (1 initial + possibly a few if tmux cursor blink causes changes)
    assert!(
        frame_count <= 5,
        "Static screen should produce few frames due to dedup, got {}",
        frame_count
    );
}

#[test]
fn test_record_list() {
    let rec_dir = tempfile::tempdir().unwrap();
    let rec_dir_str = rec_dir.path().to_string_lossy().to_string();

    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    // Create two recordings in different groups
    run_raw_ok(&[
        "record", "start",
        "--session", &s.name,
        "--group", "group-a",
        "--label", "first",
        "--dir", &rec_dir_str,
    ]);
    std::thread::sleep(std::time::Duration::from_millis(300));
    run_raw_ok(&["record", "stop", "--session", &s.name]);

    std::thread::sleep(std::time::Duration::from_millis(100));

    run_raw_ok(&[
        "record", "start",
        "--session", &s.name,
        "--group", "group-b",
        "--label", "second",
        "--dir", &rec_dir_str,
    ]);
    std::thread::sleep(std::time::Duration::from_millis(300));
    run_raw_ok(&["record", "stop", "--session", &s.name]);

    // List recordings as JSON
    let output = run_raw_ok(&["record", "list", "--dir", &rec_dir_str, "--json"]);
    let recordings: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(recordings.is_array());

    let arr = recordings.as_array().unwrap();
    assert_eq!(arr.len(), 2, "Should have 2 recordings");

    // Check that both groups are present
    let groups: Vec<&str> = arr.iter().map(|r| r["group"].as_str().unwrap()).collect();
    assert!(groups.contains(&"group-a"));
    assert!(groups.contains(&"group-b"));
}

#[test]
fn test_record_group_label_structure() {
    let rec_dir = tempfile::tempdir().unwrap();
    let rec_dir_str = rec_dir.path().to_string_lossy().to_string();

    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    // Record "before"
    run_raw_ok(&[
        "record", "start",
        "--session", &s.name,
        "--group", "fix-123",
        "--label", "before",
        "--dir", &rec_dir_str,
    ]);
    std::thread::sleep(std::time::Duration::from_millis(300));
    run_raw_ok(&["record", "stop", "--session", &s.name]);

    std::thread::sleep(std::time::Duration::from_millis(100));

    // Record "after"
    run_raw_ok(&[
        "record", "start",
        "--session", &s.name,
        "--group", "fix-123",
        "--label", "after",
        "--dir", &rec_dir_str,
    ]);
    std::thread::sleep(std::time::Duration::from_millis(300));
    run_raw_ok(&["record", "stop", "--session", &s.name]);

    // Both should be under the same group directory
    let group_dir = rec_dir.path().join("fix-123");
    assert!(group_dir.exists());

    let entries: Vec<_> = std::fs::read_dir(&group_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 2, "Should have 2 recordings in same group");

    // Check that labels are different
    let mut labels = Vec::new();
    for entry in &entries {
        let meta_str = std::fs::read_to_string(entry.path().join("meta.json")).unwrap();
        let meta: serde_json::Value = serde_json::from_str(&meta_str).unwrap();
        labels.push(meta["label"].as_str().unwrap().to_string());
    }
    labels.sort();
    assert_eq!(labels, vec!["after", "before"]);
}
