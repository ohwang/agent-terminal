#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_wait_text() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    s.run_ok(&["send", "j"]);
    let out = s.run_ok(&["wait", "--text", "Count: 1", "--timeout", "3000"]);
    assert!(out.contains("Count: 1"));
}

#[test]
fn test_wait_text_timeout() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    // Wait for text that won't appear
    let out = s.run_fail(&["wait", "--text", "NONEXISTENT", "--timeout", "1000"]);
    assert!(out.contains("timed out"));
}

#[test]
fn test_wait_text_gone() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    // "Count: 0" is present
    s.run_ok(&["send", "j"]);
    // Wait for "Count: 0" to disappear (it will be replaced by "Count: 1")
    let out = s.run_ok(&["wait", "--text-gone", "Count: 0", "--timeout", "3000"]);
    assert!(out.contains("Count: 1"));
}

#[test]
fn test_wait_stable() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let out = s.run_ok(&["wait", "--stable", "300", "--timeout", "5000"]);
    assert!(out.contains("Count:"));
}

#[test]
fn test_wait_regex() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let out = s.run_ok(&["wait", "--regex", r"Count: \d+", "--timeout", "3000"]);
    assert!(out.contains("Count:"));
}

#[test]
fn test_wait_hard_ms() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let start = std::time::Instant::now();
    s.run_ok(&["wait", "500"]);
    let elapsed = start.elapsed().as_millis();
    assert!(
        elapsed >= 400,
        "Should have waited at least 400ms, got {}ms",
        elapsed
    );
}

#[test]
fn test_wait_cursor() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    // Get current cursor position via snapshot --json
    let snap = s.run_ok(&["snapshot", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&snap).unwrap();
    let cursor_row = json["cursor"]["row"].as_u64().unwrap();
    let cursor_col = json["cursor"]["col"].as_u64().unwrap();

    // Wait for cursor at its current position -- should succeed immediately
    let cursor_str = format!("{},{}", cursor_row, cursor_col);
    let out = s.run_ok(&["wait", "--cursor", &cursor_str, "--timeout", "3000"]);
    assert!(!out.is_empty());
}

#[test]
fn test_wait_cursor_timeout() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    // Wait for cursor at a position it will never reach
    let out = s.run_fail(&["wait", "--cursor", "99,99", "--timeout", "1000"]);
    assert!(out.contains("timed out"));
}

#[test]
fn test_wait_cursor_after_interaction() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    // Get cursor position before interaction
    let snap_before = s.run_ok(&["snapshot", "--json"]);
    let json_before: serde_json::Value = serde_json::from_str(&snap_before).unwrap();
    let row_before = json_before["cursor"]["row"].as_u64().unwrap();
    let col_before = json_before["cursor"]["col"].as_u64().unwrap();

    // Increment the counter (re-renders screen, cursor goes back to same position)
    s.run_ok(&["send", "j"]);
    s.run_ok(&["wait", "--text", "Count: 1", "--timeout", "3000"]);

    // Cursor should be at the same position after re-render
    let cursor_str = format!("{},{}", row_before, col_before);
    let out = s.run_ok(&["wait", "--cursor", &cursor_str, "--timeout", "3000"]);
    assert!(out.contains("Count: 1"));
}
