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
