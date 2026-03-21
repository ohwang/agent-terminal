#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_resize_changes_size() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    // Resize to 40x10
    s.run_ok(&["resize", "40", "10"]);
    std::thread::sleep(std::time::Duration::from_millis(300));

    let snap = s.run_ok(&["snapshot"]);
    assert!(snap.contains("40x10"));
}

#[test]
fn test_resize_back_to_default() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    s.run_ok(&["resize", "40", "10"]);
    std::thread::sleep(std::time::Duration::from_millis(200));
    s.run_ok(&["resize", "80", "24"]);
    std::thread::sleep(std::time::Duration::from_millis(200));

    let snap = s.run_ok(&["snapshot"]);
    assert!(snap.contains("80x24"));
}
