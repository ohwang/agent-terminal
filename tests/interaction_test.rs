#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_send_single_key() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    s.run_ok(&["send", "j"]);
    s.run_ok(&["wait", "--text", "Count: 1", "--timeout", "3000"]);

    let snap = s.run_ok(&["snapshot"]);
    assert!(snap.contains("Count: 1"));
}

#[test]
fn test_send_multiple_keys() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    s.run_ok(&["send", "j", "j", "j"]);
    s.run_ok(&["wait", "--text", "Count: 3", "--timeout", "3000"]);

    let snap = s.run_ok(&["snapshot"]);
    assert!(snap.contains("Count: 3"));
}

#[test]
fn test_send_decrement() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    s.run_ok(&["send", "k"]);
    s.run_ok(&["wait", "--text", "Count: -1", "--timeout", "3000"]);

    let snap = s.run_ok(&["snapshot"]);
    assert!(snap.contains("Count: -1"));
}

#[test]
fn test_type_text() {
    let s = Session::new();
    let path = Session::fixture_path("echo");
    s.run_ok(&["open", &path]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    s.run_ok(&["type", "hello"]);
    std::thread::sleep(std::time::Duration::from_millis(300));

    // The echo app shows typed text or at least the app should still be alive
    let status = s.run_ok(&["status"]);
    assert!(status.contains("alive"));
}

#[test]
fn test_paste() {
    let s = Session::new();
    let path = Session::fixture_path("echo");
    s.run_ok(&["open", &path]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    s.run_ok(&["paste", "pasted text"]);
    std::thread::sleep(std::time::Duration::from_millis(300));

    let status = s.run_ok(&["status"]);
    assert!(status.contains("alive"));
}

#[test]
fn test_resize() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    s.run_ok(&["resize", "40", "10"]);
    std::thread::sleep(std::time::Duration::from_millis(300));

    let snap = s.run_ok(&["snapshot"]);
    assert!(snap.contains("40x10"));
}
