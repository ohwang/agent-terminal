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
