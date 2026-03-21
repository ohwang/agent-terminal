#[path = "common/mod.rs"]
mod common;

#[test]
fn test_doctor_passes() {
    let s = common::Session::new();
    let out = s.run_ok(&["doctor"]);
    assert!(out.contains("\u{2713}") || out.contains("OK") || out.contains("passed"));
}
