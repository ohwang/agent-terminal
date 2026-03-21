#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_scrollback_basic() {
    let s = Session::new();
    s.open_fixture_wait("slow", "Frame:");

    // Wait for some frames to accumulate
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let out = s.run_ok(&["scrollback", "--lines", "20"]);
    assert!(out.contains("Frame:"), "Scrollback should contain Frame text");
}

#[test]
fn test_scrollback_search() {
    let s = Session::new();
    s.open_fixture_wait("slow", "Frame:");

    std::thread::sleep(std::time::Duration::from_millis(1000));

    let out = s.run_ok(&["scrollback", "--search", "Frame"]);
    assert!(out.contains("Frame"), "Search should find Frame text");
}
