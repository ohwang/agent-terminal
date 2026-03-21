#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_click_left() {
    let s = Session::new();
    s.open_fixture_wait("mouse", "Click anywhere");

    // Click at row 5, col 10
    s.run_ok(&["click", "5", "10"]);
    s.run_ok(&["wait", "--text", "Clicked:", "--timeout", "3000"]);

    let snap = s.run_ok(&["snapshot"]);
    assert!(
        snap.contains("Clicked:"),
        "Should show click coordinates: {}",
        snap
    );
}

#[test]
fn test_click_right() {
    let s = Session::new();
    s.open_fixture_wait("mouse", "Click anywhere");

    s.run_ok(&["click", "3", "5", "--right"]);
    s.run_ok(&["wait", "--text", "Clicked:", "--timeout", "3000"]);

    let snap = s.run_ok(&["snapshot"]);
    assert!(
        snap.contains("Clicked:"),
        "Right click should register: {}",
        snap
    );
}

#[test]
fn test_click_double() {
    let s = Session::new();
    s.open_fixture_wait("mouse", "Click anywhere");

    s.run_ok(&["click", "3", "5", "--double"]);
    s.run_ok(&["wait", "--text", "Clicked:", "--timeout", "3000"]);

    let snap = s.run_ok(&["snapshot"]);
    assert!(
        snap.contains("Clicked:"),
        "Double click should register: {}",
        snap
    );
}

#[test]
fn test_click_coordinates() {
    let s = Session::new();
    s.open_fixture_wait("mouse", "Click anywhere");

    // Click at a specific position and verify the coordinates are reported
    s.run_ok(&["click", "7", "15"]);
    s.run_ok(&["wait", "--text", "Clicked:", "--timeout", "3000"]);

    let snap = s.run_ok(&["snapshot"]);
    assert!(
        snap.contains("row=7") && snap.contains("col=15"),
        "Should report row=7 col=15, got: {}",
        snap
    );
}

#[test]
fn test_scroll_wheel_up() {
    let s = Session::new();
    s.open_fixture_wait("mouse", "Click anywhere");

    // Scroll wheel up - the mouse fixture only tracks button presses (not scroll),
    // but the command should succeed without error
    s.run_ok(&["scroll-wheel", "up", "5", "10"]);

    // Verify app is still alive
    let status = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&status).expect("invalid JSON");
    assert_eq!(json["alive"], true);
}

#[test]
fn test_scroll_wheel_down() {
    let s = Session::new();
    s.open_fixture_wait("mouse", "Click anywhere");

    s.run_ok(&["scroll-wheel", "down", "5", "10"]);

    // Verify app is still alive
    let status = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&status).expect("invalid JSON");
    assert_eq!(json["alive"], true);
}

#[test]
fn test_scroll_wheel_invalid_direction() {
    let s = Session::new();
    s.open_fixture_wait("mouse", "Click anywhere");

    let err = s.run_fail(&["scroll-wheel", "left", "5", "10"]);
    assert!(
        err.contains("Unknown scroll direction"),
        "Should reject invalid direction: {}",
        err
    );
}

#[test]
fn test_drag() {
    let s = Session::new();
    s.open_fixture_wait("mouse", "Click anywhere");

    // Drag from (2,2) to (5,10)
    s.run_ok(&["drag", "2", "2", "5", "10"]);

    // The drag sends a press at start and release at end.
    // The fixture processes press events, so it should show the start position.
    s.run_ok(&["wait", "--text", "Clicked:", "--timeout", "3000"]);

    let snap = s.run_ok(&["snapshot"]);
    assert!(
        snap.contains("Clicked:"),
        "Drag start should register as click: {}",
        snap
    );
}

#[test]
fn test_click_then_quit() {
    let s = Session::new();
    s.open_fixture_wait("mouse", "Click anywhere");

    // Click, verify, then send 'q' to quit the app
    s.run_ok(&["click", "4", "8"]);
    s.run_ok(&["wait", "--text", "Clicked:", "--timeout", "3000"]);

    // Send 'q' to quit the mouse app
    s.run_ok(&["send", "q"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Process should be dead after quitting
    let status = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&status).expect("invalid JSON");
    assert_eq!(json["alive"], false);
}
