#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_open_and_close() {
    let s = Session::new();
    let path = Session::fixture_path("echo");
    let out = s.run_ok(&["open", &path]);
    assert!(!out.is_empty()); // prints session name

    // Verify session exists
    let status = s.run_ok(&["status"]);
    assert!(status.contains("alive"));

    // Close
    s.run_ok(&["close"]);
}

#[test]
fn test_open_with_env() {
    let s = Session::new();
    let path = Session::fixture_path("echo");
    s.run_ok(&["open", &path, "--env", "TEST_VAR=hello123"]);
    // Session should be created
    let status = s.run_ok(&["status"]);
    assert!(status.contains("alive") || status.contains("pid"));
}

#[test]
fn test_open_with_size() {
    let s = Session::new();
    let path = Session::fixture_path("counter");
    s.run_ok(&["open", &path, "--size", "40x10"]);
    std::thread::sleep(std::time::Duration::from_millis(500));
    let snap = s.run_ok(&["snapshot"]);
    assert!(snap.contains("40x10") || snap.contains("size: 40x10"));
}

#[test]
fn test_list_sessions() {
    let s = Session::new();
    let path = Session::fixture_path("echo");
    s.run_ok(&["open", &path]);

    let out = s.run_ok(&["list"]);
    assert!(out.contains(&s.name));
}

#[test]
fn test_close_nonexistent_session() {
    let s = Session::new();
    let out = s.run_fail(&["close"]);
    assert!(
        out.contains("does not exist")
            || out.contains("not found")
            || out.contains("error")
            || out.contains("ERROR")
    );
}

#[test]
fn test_status_json() {
    let s = Session::new();
    let path = Session::fixture_path("counter");
    s.run_ok(&["open", &path]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    let out = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    assert_eq!(json["alive"], true);
    assert!(json["pid"].is_number() || json["pid"].is_null());
}
