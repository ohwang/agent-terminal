#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_assert_text_pass() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    let out = s.run_ok(&["assert", "--text", "Count: 0"]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_text_fail() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    let out = s.run_fail(&["assert", "--text", "Count: 99"]);
    assert!(out.contains("FAIL") || out.contains("not found"));
}

#[test]
fn test_assert_no_text_pass() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    let out = s.run_ok(&["assert", "--no-text", "ERROR"]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_no_text_fail() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    let out = s.run_fail(&["assert", "--no-text", "Count: 0"]);
    assert!(out.contains("FAIL") || out.contains("found"));
}

#[test]
fn test_assert_row() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count: 0");

    // Row 1 should have "Count: 0"
    let out = s.run_ok(&["assert", "--row", "1", "--row-text", "Count: 0"]);
    assert!(out.contains("PASS"));
}
