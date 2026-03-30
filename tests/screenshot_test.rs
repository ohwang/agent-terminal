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

#[test]
fn test_screenshot_html_annotate() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("annotate.html");
    let path_str = path.to_string_lossy().to_string();

    s.run_ok(&["screenshot", "--html", "--annotate", "--path", &path_str]);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("row-num"),
        "Annotated HTML should have row numbers"
    );
    assert!(
        content.contains("col-ruler"),
        "Annotated HTML should have a column ruler"
    );
}

#[test]
fn test_screenshot_html_light_theme() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("light.html");
    let path_str = path.to_string_lossy().to_string();

    s.run_ok(&[
        "screenshot",
        "--html",
        "--theme",
        "light",
        "--path",
        &path_str,
    ]);

    let content = std::fs::read_to_string(&path).unwrap();
    // Light theme should use white/light background
    assert!(
        content.contains("#ffffff") || content.contains("white"),
        "Light theme should have white background"
    );
}

#[test]
fn test_screenshot_png_annotate() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let dir = tempfile::tempdir().unwrap();
    let plain_path = dir.path().join("plain.png");
    let plain_str = plain_path.to_string_lossy().to_string();
    let annotate_path = dir.path().join("annotate.png");
    let annotate_str = annotate_path.to_string_lossy().to_string();

    // Capture without annotation
    s.run_ok(&["screenshot", "--path", &plain_str]);
    // Capture with annotation (adds gutter for row numbers)
    s.run_ok(&["screenshot", "--annotate", "--path", &annotate_str]);

    assert!(plain_path.exists());
    assert!(annotate_path.exists());

    let plain_size = std::fs::metadata(&plain_path).unwrap().len();
    let annotate_size = std::fs::metadata(&annotate_path).unwrap().len();
    // Annotated image should be larger than non-annotated due to gutter
    assert!(
        annotate_size > plain_size,
        "Annotated image ({} bytes) should be larger than plain ({} bytes)",
        annotate_size,
        plain_size
    );
}
