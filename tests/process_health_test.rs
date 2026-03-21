#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_status_alive() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let out = s.run_ok(&["status"]);
    assert!(out.contains("alive"));
}

#[test]
fn test_status_json_alive() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let out = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    assert_eq!(json["alive"], true);
}

#[test]
fn test_crash_detection() {
    let s = Session::new();
    let path = Session::fixture_path("crash");
    s.run_ok(&["open", &path]);

    // Wait for crash (fixture exits after ~500ms)
    std::thread::sleep(std::time::Duration::from_millis(1500));

    // Status should show not alive
    let out = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    assert_eq!(json["alive"], false);
}

#[test]
fn test_exit_code() {
    let s = Session::new();
    let path = Session::fixture_path("crash");
    s.run_ok(&["open", &path]);

    std::thread::sleep(std::time::Duration::from_millis(1500));

    let out = s.run_ok(&["exit-code"]);
    assert!(out.contains("42"));
}

#[test]
fn test_logs_stderr() {
    let s = Session::new();
    let path = Session::fixture_path("crash");
    s.run_ok(&["open", &path]);

    std::thread::sleep(std::time::Duration::from_millis(1500));

    let out = s.run_ok(&["logs", "--stderr"]);
    assert!(out.contains("CRASHING"));
}
