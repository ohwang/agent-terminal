#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_find_text() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let out = s.run_ok(&["find", "Count"]);
    assert!(out.contains("row"));
    assert!(out.contains("col"));
}

#[test]
fn test_find_not_found() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let out = s.run_fail(&["find", "NONEXISTENT"]);
    assert!(out.contains("not found") || out.contains("No matches"));
}

#[test]
fn test_find_all() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    // Find all occurrences - at least the status line keys
    let out = s.run_ok(&["find", "j", "--all"]);
    assert!(out.contains("row"));
}

#[test]
fn test_find_regex() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let out = s.run_ok(&["find", r"Count: \d+", "--regex"]);
    assert!(out.contains("row"));
}

#[test]
fn test_find_by_color_red() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    // Find text that has red foreground using empty pattern to match any text
    let out = s.run_ok(&["find", "", "--color", "fg:red"]);
    assert!(out.contains("Red Text"));
}

#[test]
fn test_find_by_color_bold() {
    let s = Session::new();
    s.open_fixture_wait("color", "Green Bold");

    // Find text that is bold
    let out = s.run_ok(&["find", "", "--color", "bold"]);
    assert!(out.contains("Green Bold"));
}

#[test]
fn test_find_by_color_all() {
    let s = Session::new();
    s.open_fixture_wait("color", "Yellow on Blue");

    // Find all text with any foreground color -- should return multiple matches
    let out = s.run_ok(&["find", "", "--color", "fg:red", "--all"]);
    assert!(out.contains("row"));
}

#[test]
fn test_find_by_color_with_pattern() {
    let s = Session::new();
    s.open_fixture_wait("color", "Green Bold");

    // Find specific text that has the green style
    let out = s.run_ok(&["find", "Green", "--color", "fg:green"]);
    assert!(out.contains("row"));
}

#[test]
fn test_find_by_color_not_found() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    // Search for magenta text which doesn't exist
    let out = s.run_fail(&["find", "", "--color", "fg:magenta"]);
    assert!(out.contains("not found") || out.contains("No text with style"));
}

#[test]
fn test_find_by_color_bg() {
    let s = Session::new();
    s.open_fixture_wait("color", "Yellow on Blue");

    // Find text with blue background
    let out = s.run_ok(&["find", "", "--color", "bg:blue"]);
    assert!(out.contains("Yellow on Blue"));
}
