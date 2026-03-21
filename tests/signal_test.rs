#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_signal_sigterm() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    // Send SIGTERM (more reliable than SIGINT for raw-mode apps)
    s.run_ok(&["signal", "SIGTERM"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Process should be dead
    let out = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    assert_eq!(json["alive"], false);
}

#[test]
fn test_signal_sigkill() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    // SIGKILL always works
    s.run_ok(&["signal", "SIGKILL"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    let out = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    assert_eq!(json["alive"], false);
}
