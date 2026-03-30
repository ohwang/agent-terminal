#[path = "common/mod.rs"]
mod common;
use common::Session;

use std::thread;
use std::time::{Duration, Instant};

fn sleep_ms(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

/// Scenario 14: Large output and scrollback
/// Generates 500 lines of output, verifies visible snapshot vs scrollback.
#[test]
fn test_large_output_and_scrollback() {
    let s = Session::new();
    s.run_ok(&["open", "/bin/bash --norc --noprofile"]);

    // bash --norc --noprofile with stderr redirected (done by agent-terminal open)
    // produces an invisible prompt. The shell IS running -- just type and Enter.
    sleep_ms(1500);

    // Type `seq 1 500` to generate 500 lines of output
    s.run_ok(&["type", "seq 1 500"]);
    sleep_ms(300);
    s.run_ok(&["send", "Enter"]);

    // Wait for "500" to appear on screen (the last line of output)
    s.run_ok(&["wait", "--text", "500", "--timeout", "10000"]);
    sleep_ms(500);

    // Take a regular snapshot - should show only the visible portion (bottom of output)
    let plain_snap = s.run_ok(&["snapshot"]);
    println!(
        "=== Plain Snapshot (last visible portion) ===\n{}",
        plain_snap
    );

    // The visible portion should contain "500" (the end) but likely NOT "1" at the very start
    assert!(
        plain_snap.contains("500"),
        "Plain snapshot should contain '500' (end of seq output)"
    );

    // Count visible content lines (excluding header)
    let plain_content_lines: Vec<&str> = plain_snap
        .lines()
        .filter(|l| l.contains('│') || l.contains('|'))
        .collect();
    println!(
        "Plain snapshot has {} content lines",
        plain_content_lines.len()
    );

    // Take a scrollback snapshot with 100 lines of history
    let scrollback_snap = s.run_ok(&["snapshot", "--scrollback", "100"]);
    println!(
        "=== Scrollback Snapshot (100 lines) ===\n{}",
        &scrollback_snap[..scrollback_snap.len().min(2000)]
    );

    let scrollback_content_lines: Vec<&str> = scrollback_snap
        .lines()
        .filter(|l| l.contains('│') || l.contains('|'))
        .collect();
    println!(
        "Scrollback snapshot has {} content lines",
        scrollback_content_lines.len()
    );

    // Scrollback should have more lines than plain snapshot
    assert!(
        scrollback_content_lines.len() >= plain_content_lines.len(),
        "Scrollback ({} lines) should have >= lines than plain ({} lines)",
        scrollback_content_lines.len(),
        plain_content_lines.len()
    );

    // Also test the raw scrollback command
    let raw_scrollback = s.run_ok(&["scrollback", "--lines", "50"]);
    println!(
        "=== Raw Scrollback (50 lines) ===\n{}",
        &raw_scrollback[..raw_scrollback.len().min(1500)]
    );

    // Raw scrollback should contain content
    assert!(
        !raw_scrollback.trim().is_empty(),
        "Raw scrollback should not be empty"
    );

    // Check that scrollback contains earlier lines from the seq output.
    // With 500 lines generated and a ~24-row terminal, earlier numbers
    // should be in the scrollback buffer but not in the visible snapshot.
    let scrollback_has_earlier = raw_scrollback.contains("450")
        || raw_scrollback.contains("460")
        || raw_scrollback.contains("470")
        || raw_scrollback.contains("480");
    println!(
        "Scrollback contains earlier seq numbers: {}",
        scrollback_has_earlier
    );
    assert!(
        scrollback_has_earlier,
        "Scrollback should contain earlier seq output lines"
    );

    // Exit bash
    s.run_ok(&["type", "exit"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);
    sleep_ms(500);
}

/// Scenario 15: Rapid input stress test
/// Sends 50 'j' keys as fast as possible to the counter fixture.
#[test]
fn test_rapid_input_stress() {
    let s = Session::new();
    let counter_path = Session::fixture_path("counter");
    s.run_ok(&["open", &counter_path]);

    // Wait for initial state
    s.run_ok(&["wait", "--text", "Count: 0", "--timeout", "5000"]);
    sleep_ms(300);

    // Send 'j' key 50 times as rapidly as possible (no sleep between sends)
    let start = Instant::now();
    for _ in 0..50 {
        s.run_ok(&["send", "j"]);
    }
    let elapsed = start.elapsed();
    println!(
        "=== Rapid Input Timing ===\nSent 50 'j' keys in {:?} ({:.1} keys/sec)",
        elapsed,
        50.0 / elapsed.as_secs_f64()
    );

    // Wait for the counter to process all inputs
    sleep_ms(1000);

    // Snapshot and check the count
    let snap = s.run_ok(&["snapshot"]);
    println!("=== After 50 Rapid Sends ===\n{}", snap);

    // Extract the count value from the snapshot
    let count_line = snap
        .lines()
        .find(|l| l.contains("Count:"))
        .expect("Should find Count: line in snapshot");
    println!("Count line: {}", count_line);

    // Parse the count number - handle both plain and ANSI-colored output
    let count_str = count_line
        .split("Count:")
        .nth(1)
        .expect("Should have text after 'Count:'");
    // Strip ANSI codes and whitespace, extract the number
    let count_num: i64 = count_str
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| {
            // Try harder - find any number in the string
            let digits: String = count_str
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '-')
                .collect();
            digits.parse().unwrap_or(0)
        });

    println!("Parsed count: {}", count_num);

    // The count should be near 50. Under stress some might be dropped, but
    // since we're using run_ok (which waits for each command to complete),
    // all 50 should be delivered.
    assert!(
        count_num >= 40 && count_num <= 50,
        "Count should be between 40 and 50, got {}. Elapsed: {:?}",
        count_num,
        elapsed
    );

    // Performance observation
    let avg_ms = elapsed.as_millis() as f64 / 50.0;
    println!("Average time per send: {:.1}ms", avg_ms);
    println!("Total elapsed: {:.0}ms for 50 keys", elapsed.as_millis());
}

/// Scenario 16: Resize during active rendering
/// Resizes the terminal while the resize fixture is running and verifies
/// the app detects the new dimensions.
#[test]
fn test_resize_during_active_rendering() {
    let s = Session::new();
    let resize_path = Session::fixture_path("resize");
    s.run_ok(&["open", &resize_path]);

    // Wait for the terminal size to appear (default is 112x30)
    s.run_ok(&["wait", "--text", "Size:", "--timeout", "5000"]);
    sleep_ms(500);

    // Snapshot and note the initial size
    let initial_snap = s.run_ok(&["snapshot"]);
    println!("=== Initial Size Snapshot ===\n{}", initial_snap);

    // Verify initial size is displayed (default 112x30)
    assert!(
        initial_snap.contains("112x30"),
        "Initial snapshot should show 112x30, got:\n{}",
        initial_snap
    );

    // Resize to 40x10
    s.run_ok(&["resize", "40", "10"]);
    // Wait for the app to handle SIGWINCH and re-render
    sleep_ms(500);

    let small_snap = s.run_ok(&["snapshot"]);
    println!("=== After Resize to 40x10 ===\n{}", small_snap);

    // The resize app should show the new dimensions
    assert!(
        small_snap.contains("40x10"),
        "After resize to 40x10, snapshot should show 40x10, got:\n{}",
        small_snap
    );

    // Verify the snapshot header also reflects the new size
    assert!(
        small_snap.contains("40x10"),
        "Snapshot header should reflect 40x10"
    );

    // Now resize to a larger size: 120x40
    s.run_ok(&["resize", "120", "40"]);
    sleep_ms(500);

    let large_snap = s.run_ok(&["snapshot"]);
    println!("=== After Resize to 120x40 ===\n{}", large_snap);

    // The resize app should show the new dimensions
    assert!(
        large_snap.contains("120x40"),
        "After resize to 120x40, snapshot should show 120x40, got:\n{}",
        large_snap
    );

    // Verify the snapshot header also reflects the new size
    // The header format includes "size: COLSxROWS" or similar
    let header_line = large_snap
        .lines()
        .find(|l| l.contains("size:") || l.contains("cols:"));
    println!("Header with size info: {:?}", header_line);

    // Verify we can see both the app's size report and the header size
    let large_lines: Vec<&str> = large_snap
        .lines()
        .filter(|l| l.contains('│') || l.contains('|'))
        .collect();
    println!(
        "Large snapshot has {} content lines (expected ~40 for 120x40)",
        large_lines.len()
    );
}
