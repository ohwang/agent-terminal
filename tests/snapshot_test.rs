#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_snapshot_plain() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let snap = s.run_ok(&["snapshot"]);
    assert!(snap.contains("Count: 0"));
    assert!(snap.contains("[j] +1"));
    assert!(snap.contains("session:"));
    // Check row numbers
    assert!(snap.contains("1│") || snap.contains("1|"));
}

#[test]
fn test_snapshot_color() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    let snap = s.run_ok(&["snapshot", "--color"]);
    assert!(snap.contains("Red Text"));
    assert!(snap.contains("[fg:red"));
    assert!(snap.contains("Green Bold"));
    assert!(snap.contains("bold"));
}

#[test]
fn test_snapshot_raw() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    let snap = s.run_ok(&["snapshot", "--raw"]);
    // Raw mode should have ANSI escapes, no row numbers
    assert!(snap.contains("Red Text"));
    assert!(!snap.contains("session:"));
}

#[test]
fn test_snapshot_ansi() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    let snap = s.run_ok(&["snapshot", "--ansi"]);
    // ANSI mode should have row numbers AND escape sequences
    assert!(snap.contains("Red Text"));
    assert!(snap.contains("session:"));
}

#[test]
fn test_snapshot_json() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let out = s.run_ok(&["snapshot", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    assert!(json["session"].is_string());
    assert!(json["size"]["cols"].is_number());
    assert!(json["size"]["rows"].is_number());
    assert!(json["cursor"]["row"].is_number());
    assert!(json["lines"].is_array());

    // Find the Count line
    let lines = json["lines"].as_array().unwrap();
    let count_line = lines.iter().find(|l| {
        l["text"]
            .as_str()
            .map(|t| t.contains("Count"))
            .unwrap_or(false)
    });
    assert!(
        count_line.is_some(),
        "Should find Count line in JSON output"
    );
}

#[test]
fn test_snapshot_json_color_spans() {
    let s = Session::new();
    s.open_fixture_wait("color", "Red Text");

    let out = s.run_ok(&["snapshot", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    let lines = json["lines"].as_array().unwrap();

    // Find the Red Text line
    let red_line = lines.iter().find(|l| {
        l["text"]
            .as_str()
            .map(|t| t.contains("Red Text"))
            .unwrap_or(false)
    });
    assert!(red_line.is_some());
    let red_line = red_line.unwrap();
    let spans = red_line["spans"].as_array().unwrap();
    assert!(!spans.is_empty());
    // Check that at least one span has fg:red
    let has_red = spans.iter().any(|s| s["fg"].as_str() == Some("red"));
    assert!(has_red, "Red Text line should have a red span");
}

#[test]
fn test_snapshot_diff() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    // Take first snapshot (establishes baseline)
    s.run_ok(&["snapshot", "--diff"]);

    // Change state
    s.run_ok(&["send", "j"]);
    s.run_ok(&["wait", "--text", "Count: 1", "--timeout", "3000"]);

    // Take diff snapshot
    let diff = s.run_ok(&["snapshot", "--diff"]);
    // Should show changes
    assert!(diff.contains("Count: 1") || diff.contains("+") || diff.contains("-"));
}
