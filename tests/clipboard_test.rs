#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_clipboard_write_read() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    s.run_ok(&["clipboard", "write", "test clipboard content"]);
    let out = s.run_ok(&["clipboard", "read"]);
    assert!(out.contains("test clipboard content"));
}

#[test]
fn test_clipboard_paste() {
    let s = Session::new();
    let path = Session::fixture_path("echo");
    s.run_ok(&["open", &path]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    s.run_ok(&["clipboard", "write", "pasted"]);
    s.run_ok(&["clipboard", "paste"]);
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Just verify the session is still alive
    let status = s.run_ok(&["status"]);
    assert!(status.contains("alive"));
}
