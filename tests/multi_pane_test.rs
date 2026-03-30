#[path = "common/mod.rs"]
mod common;
use common::Session;

use std::process::Command;

/// Query the pane indices for a session. Returns a list of "window.pane" strings
/// in order of pane creation (by pane index).
fn list_pane_targets(session_name: &str) -> Vec<String> {
    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-t",
            session_name,
            "-F",
            "#{window_index}.#{pane_index}",
        ])
        .output()
        .expect("failed to list panes");
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .trim()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[test]
fn test_open_pane_requires_existing_session() {
    let s = Session::new();
    let path = Session::fixture_path("echo");

    // Opening with --pane on a non-existent session should fail
    let err = s.run_fail(&["open", &path, "--pane", "second"]);
    assert!(
        err.contains("does not exist") || err.contains("cannot split"),
        "Should fail when session doesn't exist: {}",
        err
    );
}

#[test]
fn test_open_pane_splits_session() {
    let s = Session::new();

    // Open first fixture (creates the session)
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    // Open second fixture in a new pane via split-window
    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Should now have two panes
    let panes = list_pane_targets(&s.name);
    assert_eq!(
        panes.len(),
        2,
        "Session should have 2 panes, got: {:?}",
        panes
    );

    // The session should still be alive
    let status = s.run_ok(&["status"]);
    assert!(
        status.contains("alive"),
        "Session should be alive: {}",
        status
    );
}

#[test]
fn test_snapshot_default_pane_after_split() {
    let s = Session::new();

    // Open first fixture
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    // Split to create second pane with echo app
    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // After split-window, the newly created pane is the active pane.
    // A snapshot without --pane targets the currently active pane,
    // which should be the echo app (the second pane).
    let snap = s.run_ok(&["snapshot"]);
    assert!(
        snap.contains("READY"),
        "Default snapshot after split should show newly active pane (echo app): {}",
        snap
    );
}

#[test]
fn test_snapshot_specific_pane() {
    let s = Session::new();

    // Open counter in the first pane
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    // Split to create second pane with echo app
    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Discover pane indices
    let panes = list_pane_targets(&s.name);
    assert_eq!(panes.len(), 2, "Expected 2 panes: {:?}", panes);

    // Target the first pane (counter)
    let snap_first = s.run_ok(&["snapshot", "--pane", &panes[0]]);
    assert!(
        snap_first.contains("Count:"),
        "First pane should show counter app: {}",
        snap_first
    );

    // Target the second pane (echo)
    let snap_second = s.run_ok(&["snapshot", "--pane", &panes[1]]);
    assert!(
        snap_second.contains("READY"),
        "Second pane should show echo app: {}",
        snap_second
    );
}

#[test]
fn test_send_keys_to_specific_pane() {
    let s = Session::new();

    // Open counter in the first pane
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count: 0", "--timeout", "5000"]);

    // Split to create second pane
    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Discover pane indices
    let panes = list_pane_targets(&s.name);
    let first_pane = &panes[0];

    // Send 'j' to the first pane (counter) to increment
    s.run_ok(&["send", "--pane", first_pane, "j"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Verify counter incremented in the first pane
    let snap = s.run_ok(&["snapshot", "--pane", first_pane]);
    assert!(
        snap.contains("Count: 1"),
        "Counter should have incremented in first pane: {}",
        snap
    );
}

#[test]
fn test_type_text_to_specific_pane() {
    let s = Session::new();

    // Open echo in the first pane
    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Split to create second pane with counter
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let panes = list_pane_targets(&s.name);
    let first_pane = &panes[0];

    // Type text into the first pane (echo)
    s.run_ok(&["type", "--pane", first_pane, "hello"]);
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Both panes should still be alive
    let status = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&status).expect("invalid JSON");
    assert_eq!(json["alive"], true);
}

#[test]
fn test_status_specific_pane() {
    let s = Session::new();

    // Open counter in the first pane
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    // Split to create second pane
    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let panes = list_pane_targets(&s.name);

    // Status for the first pane
    let status_first = s.run_ok(&["status", "--pane", &panes[0], "--json"]);
    let json_first: serde_json::Value =
        serde_json::from_str(&status_first).expect("invalid JSON for first pane");
    assert_eq!(json_first["alive"], true, "First pane should be alive");

    // Status for the second pane
    let status_second = s.run_ok(&["status", "--pane", &panes[1], "--json"]);
    let json_second: serde_json::Value =
        serde_json::from_str(&status_second).expect("invalid JSON for second pane");
    assert_eq!(json_second["alive"], true, "Second pane should be alive");
}

#[test]
fn test_panes_are_independent() {
    let s = Session::new();

    // Open counter in the first pane
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count: 0", "--timeout", "5000"]);

    // Split to create second pane with echo
    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let panes = list_pane_targets(&s.name);
    let first_pane = &panes[0];
    let second_pane = &panes[1];

    // Interact with counter pane: increment three times
    s.run_ok(&["send", "--pane", first_pane, "j"]);
    s.run_ok(&["send", "--pane", first_pane, "j"]);
    s.run_ok(&["send", "--pane", first_pane, "j"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Verify counter shows 3
    let snap_counter = s.run_ok(&["snapshot", "--pane", first_pane]);
    assert!(
        snap_counter.contains("Count: 3"),
        "Counter pane should show Count: 3: {}",
        snap_counter
    );

    // Verify echo pane is unaffected (still shows READY)
    let snap_echo = s.run_ok(&["snapshot", "--pane", second_pane]);
    assert!(
        snap_echo.contains("READY"),
        "Echo pane should still show READY: {}",
        snap_echo
    );
}

#[test]
fn test_snapshot_window_captures_all_panes() {
    let s = Session::new();

    // Open counter in the first pane
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    // Split to create second pane with echo
    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Snapshot with --window should contain content from both panes
    let snap = s.run_ok(&["snapshot", "--window"]);
    assert!(
        snap.contains("Count:"),
        "Window snapshot should include counter pane content: {}",
        snap
    );
    assert!(
        snap.contains("READY"),
        "Window snapshot should include echo pane content: {}",
        snap
    );
    assert!(
        snap.contains("panes: 2"),
        "Window snapshot header should show pane count: {}",
        snap
    );
}

#[test]
fn test_snapshot_window_json() {
    let s = Session::new();

    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let snap = s.run_ok(&["snapshot", "--window", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&snap).expect("invalid JSON");

    // Should have window_size and panes array
    assert!(
        json["window_size"].is_object(),
        "Should have window_size: {}",
        snap
    );
    assert!(
        json["panes"].is_array(),
        "Should have panes array: {}",
        snap
    );
    assert_eq!(
        json["panes"].as_array().unwrap().len(),
        2,
        "Should have 2 panes: {}",
        snap
    );

    // Each pane should have layout info
    let pane0 = &json["panes"][0];
    assert!(pane0["pane_id"].is_string(), "Pane should have pane_id");
    assert!(pane0["left"].is_number(), "Pane should have left");
    assert!(pane0["width"].is_number(), "Pane should have width");
    assert!(pane0["lines"].is_array(), "Pane should have lines");
}

#[test]
fn test_snapshot_window_single_pane_fallback() {
    let s = Session::new();

    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    // --window with a single pane should still work (falls back to normal snapshot)
    let snap = s.run_ok(&["snapshot", "--window"]);
    assert!(
        snap.contains("Count:"),
        "Single-pane window snapshot should show content: {}",
        snap
    );
}

#[test]
fn test_screenshot_window_html() {
    let s = Session::new();

    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let tmp = format!(
        "/tmp/agent-terminal-test-window-{}.html",
        std::process::id()
    );
    let out = s.run_ok(&["screenshot", "--window", "--html", "--path", &tmp]);
    assert!(out.contains("Screenshot saved to"), "Should save: {}", out);

    let html = std::fs::read_to_string(&tmp).expect("should read HTML file");
    assert!(html.contains("pane"), "HTML should contain pane elements");
    assert!(html.contains("window"), "HTML should reference window");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_status_shows_pane_info() {
    let s = Session::new();

    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Plain text status should show pane info
    let status = s.run_ok(&["status"]);
    assert!(
        status.contains("Panes:"),
        "Status should show pane count: {}",
        status
    );
    assert!(
        status.contains("--window"),
        "Status should include hint about --window: {}",
        status
    );

    // JSON status should include panes array
    let json_status = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&json_status).expect("invalid JSON");
    assert!(
        json["panes"].is_array(),
        "JSON status should have panes array: {}",
        json_status
    );
    assert_eq!(
        json["panes"].as_array().unwrap().len(),
        2,
        "Should have 2 panes in JSON: {}",
        json_status
    );
}

#[test]
fn test_quit_one_pane_other_survives() {
    let s = Session::new();

    // Open counter in the first pane
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);
    s.run_ok(&["wait", "--text", "Count:", "--timeout", "5000"]);

    // Split to create second pane with echo
    let echo_path = Session::fixture_path("echo");
    s.run_ok(&["open", &echo_path, "--pane", "second"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let panes = list_pane_targets(&s.name);
    let first_pane = &panes[0];
    let second_pane = &panes[1];

    // Quit the echo app in the second pane
    s.run_ok(&["send", "--pane", second_pane, "q"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // The first pane (counter) should still be alive
    let status = s.run_ok(&["status", "--pane", first_pane, "--json"]);
    let json: serde_json::Value = serde_json::from_str(&status).expect("invalid JSON");
    assert_eq!(
        json["alive"], true,
        "Counter pane should survive after echo quits"
    );
}
