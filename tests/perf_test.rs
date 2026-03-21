#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_perf_latency() {
    let s = Session::new();
    s.open_fixture_wait("counter", "Count:");

    let out = s.run_ok(&["perf", "latency", "--key", "j", "--samples", "3", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");

    assert!(json["mean_ms"].is_number());
    assert!(json["min_ms"].is_number());
    assert!(json["max_ms"].is_number());
    assert!(json["p95_ms"].is_number());
    assert_eq!(json["samples"], 3);

    let mean = json["mean_ms"].as_f64().unwrap();
    assert!(mean < 1000.0, "Latency should be under 1s, got {}ms", mean);
}

#[test]
fn test_perf_fps_duration() {
    let s = Session::new();
    s.open_fixture_wait("slow", "Frame:");

    let out = s.run_ok(&["perf", "fps", "--duration", "1500"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");

    assert!(json["fps"].is_number());
    let fps = json["fps"].as_f64().unwrap();
    // Slow fixture updates every 100ms, so FPS should be around 10
    assert!(fps > 1.0, "FPS should be > 1 for slow fixture, got {}", fps);
    assert!(json["frame_count"].as_u64().unwrap() > 0);
}

#[test]
fn test_perf_start_stop() {
    let s = Session::new();
    s.open_fixture_wait("slow", "Frame:");

    s.run_ok(&["perf", "start"]);
    std::thread::sleep(std::time::Duration::from_millis(1500));
    let out = s.run_ok(&["perf", "stop", "--json"]);

    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    assert!(json["fps"].is_number());
}
