#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_screenshot_html() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.html");
    let path_str = path.to_string_lossy().to_string();

    s.run_ok(&["screenshot", "--html", "--path", &path_str]);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<html>") || content.contains("<!DOCTYPE"));
    assert!(content.contains("terminal"));
    assert!(content.contains("Red Text") || content.contains("color"));
}

#[test]
fn test_screenshot_png() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.png");
    let path_str = path.to_string_lossy().to_string();

    s.run_ok(&["screenshot", "--path", &path_str]);
    assert!(path.exists());
    assert!(std::fs::metadata(&path).unwrap().len() > 100);
}
