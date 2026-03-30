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

#[test]
fn test_assert_cursor_row() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    // Use snapshot --json to discover the actual cursor row
    let snap = s.run_ok(&["snapshot", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&snap).unwrap();
    let cursor_row = json["cursor"]["row"].as_u64().unwrap();

    // Assert that cursor-row matches (cursor-row uses 0-indexed tmux value)
    let out = s.run_ok(&["assert", "--cursor-row", &cursor_row.to_string()]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_cursor_row_fail() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    // Assert cursor is on a row it definitely is not (row 99)
    let out = s.run_fail(&["assert", "--cursor-row", "99"]);
    assert!(out.contains("FAIL"));
}

#[test]
fn test_assert_color_style_red() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    // Row 1 should have red foreground
    let out = s.run_ok(&["assert", "--color", "1", "--color-style", "fg:red"]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_color_style_green_bold() {
    let s = Session::new();
    s.open_fixture_wait("color", "Green Bold");

    // Row 2 should have green bold
    let out = s.run_ok(&["assert", "--color", "2", "--color-style", "fg:green,bold"]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_color_style_blue_underline() {
    let s = Session::new();
    s.open_fixture_wait("color", "Blue Underline");

    // Row 3 should have blue underline
    let out = s.run_ok(&[
        "assert",
        "--color",
        "3",
        "--color-style",
        "fg:blue,underline",
    ]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_color_style_reverse() {
    let s = Session::new();
    s.open_fixture_wait("color", "Reverse Video");

    // Row 4 should have reverse video
    let out = s.run_ok(&["assert", "--color", "4", "--color-style", "reverse"]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_color_style_yellow_on_blue() {
    let s = Session::new();
    s.open_fixture_wait("color", "Yellow on Blue");

    // Row 6 should have yellow foreground on blue background
    let out = s.run_ok(&[
        "assert",
        "--color",
        "6",
        "--color-style",
        "fg:yellow,bg:blue",
    ]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_color_style_fail_wrong_color() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    // Row 1 is red, not blue -- should fail
    let out = s.run_fail(&["assert", "--color", "1", "--color-style", "fg:blue"]);
    assert!(out.contains("FAIL"));
}

#[test]
fn test_assert_style_text() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    // "Red Text" should be styled with red foreground
    let out = s.run_ok(&["assert", "--style", "Red Text", "--style-check", "fg:red"]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_style_text_bold() {
    let s = Session::new();
    s.open_fixture_wait("color", "Green Bold");

    // "Green Bold" should have bold + green
    let out = s.run_ok(&[
        "assert",
        "--style",
        "Green Bold",
        "--style-check",
        "fg:green,bold",
    ]);
    assert!(out.contains("PASS"));
}

#[test]
fn test_assert_style_text_fail_wrong_style() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    // "Red Text" is red, not bold green -- should fail
    let out = s.run_fail(&[
        "assert",
        "--style",
        "Red Text",
        "--style-check",
        "fg:green,bold",
    ]);
    assert!(out.contains("FAIL"));
}

#[test]
fn test_assert_style_text_not_found() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    // Text that doesn't exist on screen
    let out = s.run_fail(&[
        "assert",
        "--style",
        "Nonexistent Text",
        "--style-check",
        "fg:red",
    ]);
    assert!(out.contains("FAIL") || out.contains("not found"));
}
