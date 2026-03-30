#[path = "common/mod.rs"]
mod common;
use common::Session;

// All matrix tests in one function to avoid session name collisions
// (test-matrix uses hardcoded at-matrix-N session names internally)
#[test]
fn test_matrix_scenarios() {
    let counter = Session::fixture_path("counter");
    let bin = Session::bin_path();

    // --- Scenario 1: all pass ---
    let output = std::process::Command::new(&bin)
        .args([
            "test-matrix",
            "--command",
            &counter,
            "--sizes",
            "80x24,40x10",
            "--terms",
            "xterm-256color",
            "--colors",
            "default",
            "--test",
            "wait --stable 500 && assert --text Count",
        ])
        .output()
        .expect("failed to run test-matrix");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Scenario 1 (all pass): stdout={}\nstderr={}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("pass"), "Should show pass: {}", stdout);
    assert!(
        stdout.contains("2/2") || stdout.contains("passed"),
        "All should pass: {}",
        stdout
    );

    // Clean up matrix output
    let _ = std::fs::remove_dir_all("./agent-terminal-matrix");

    // --- Scenario 2: assertion failure ---
    let output = std::process::Command::new(&bin)
        .args([
            "test-matrix",
            "--command",
            &counter,
            "--sizes",
            "80x24",
            "--terms",
            "xterm-256color",
            "--colors",
            "default",
            "--test",
            "wait --stable 500 && assert --text NONEXISTENT",
        ])
        .output()
        .expect("failed to run test-matrix");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "Scenario 2 should fail");
    assert!(
        stdout.contains("FAIL") || stdout.contains("failed"),
        "Should show failure: {}",
        stdout
    );

    let _ = std::fs::remove_dir_all("./agent-terminal-matrix");

    // --- Scenario 3: multiple TERM values ---
    let output = std::process::Command::new(&bin)
        .args([
            "test-matrix",
            "--command",
            &counter,
            "--sizes",
            "80x24",
            "--terms",
            "xterm-256color,xterm",
            "--colors",
            "default",
            "--test",
            "wait --stable 500 && assert --text Count",
        ])
        .output()
        .expect("failed to run test-matrix");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Scenario 3 (multiple terms): stdout={}\nstderr={}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("2") && stdout.contains("pass"),
        "Should test 2 combinations: {}",
        stdout
    );

    let _ = std::fs::remove_dir_all("./agent-terminal-matrix");
}
