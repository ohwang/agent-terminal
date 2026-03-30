#[path = "common/mod.rs"]
mod common;
use common::Session;

use std::process::Command;
use std::thread;
use std::time::Duration;

fn sleep_ms(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

/// Query the pane indices for a session. Returns a list of "window.pane" strings
/// in order of pane creation (by pane index).
fn list_pane_targets(session_name: &str) -> Vec<String> {
    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-t",
            session_name,
            "-F",
            "#{window_index}.#{pane_index}",
        ])
        .output()
        .expect("failed to list panes");
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .trim()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Helper: open bash and wait until it's responsive by sending a marker command.
///
/// Agent-terminal wraps commands with `2>file` stderr redirect, which causes
/// bash to suppress its prompt (PS1 is written to stderr). We work around this
/// by typing an echo command and waiting for its stdout output as a readiness
/// signal.
///
/// If `interactive` is true, uses `-i` flag to force job control (needed for
/// Ctrl+C to work correctly with the stderr redirect wrapper).
fn open_bash_and_wait(s: &Session, interactive: bool) {
    let cmd = if interactive {
        "/bin/bash --norc --noprofile -i"
    } else {
        "/bin/bash --norc --noprofile"
    };
    s.run_ok(&["open", cmd]);
    sleep_ms(1000);
    s.run_ok(&["type", "echo BASH_READY"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);
    s.run_ok(&["wait", "--text", "BASH_READY", "--timeout", "5000"]);
    sleep_ms(300);
}

/// Helper: send a command to a specific bash pane.
fn bash_exec_in_pane(s: &Session, pane: &str, command: &str) {
    s.run_ok(&["type", "--pane", pane, command]);
    sleep_ms(200);
    s.run_ok(&["send", "--pane", pane, "Enter"]);
}

/// Scenario 17: Ctrl+C signal handling in bash
///
/// Verifies that sending Ctrl+C to bash while a foreground process (sleep 300)
/// is running will interrupt the process and return control to bash, and that
/// bash itself survives the signal and can still execute commands.
///
/// NOTE: Requires `-i` flag for bash because agent-terminal redirects stderr
/// (`2>file` in the wrapper). Without `-i`, bash disables job control when
/// stderr is not a terminal, causing SIGINT to propagate to the entire process
/// group (including the `sh -c` wrapper), which kills the tmux session.
/// With `-i`, bash forces interactive mode with job control, so SIGINT only
/// reaches the foreground child process (sleep).
#[test]
fn test_ctrl_c_signal_handling_in_bash() {
    let s = Session::new();

    // Open bash in interactive mode (needed for Ctrl+C to work with stderr redirect)
    open_bash_and_wait(&s, true);

    // Start a long-running sleep in the foreground
    s.run_ok(&["type", "sleep 300"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);

    // Wait for sleep to start
    sleep_ms(500);

    // Snapshot while sleep is running.
    // With stderr redirected and `-i`, the typed command is not echoed back,
    // so the snapshot during sleep shows either blank or minimal content.
    // The key observation: the session is alive and no new output has appeared.
    let snap_during = s.run_ok(&["snapshot"]);
    println!("=== Snapshot during sleep 300 ===\n{}", snap_during);

    // Verify session is alive while sleep runs
    let status_during = s.run_ok(&["status", "--json"]);
    let json_during: serde_json::Value =
        serde_json::from_str(&status_during).expect("invalid JSON");
    assert_eq!(
        json_during["alive"], true,
        "Session should be alive while sleep runs"
    );

    // Send Ctrl+C to interrupt the sleep
    s.run_ok(&["send", "C-c"]);
    sleep_ms(500);

    // Verify bash survived: type a command and check its stdout output.
    // Since the prompt and typed chars are invisible (stderr redirect), we
    // verify recovery by checking command output appears on stdout.
    s.run_ok(&["type", "echo still_alive"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);

    // Wait for the output
    s.run_ok(&["wait", "--text", "still_alive", "--timeout", "5000"]);
    sleep_ms(500);

    let snap_after = s.run_ok(&["snapshot"]);
    println!("=== Snapshot after Ctrl+C recovery ===\n{}", snap_after);
    assert!(
        snap_after.contains("still_alive"),
        "Expected 'still_alive' after Ctrl+C recovery, got:\n{}",
        snap_after
    );

    // Verify the session is still alive via status --json
    let status_after = s.run_ok(&["status", "--json"]);
    let json_after: serde_json::Value = serde_json::from_str(&status_after).expect("invalid JSON");
    assert_eq!(
        json_after["alive"], true,
        "Bash session should still be alive after Ctrl+C"
    );

    // Additional verification: run another command to prove bash is fully functional
    s.run_ok(&["type", "echo second_command_works"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);
    s.run_ok(&[
        "wait",
        "--text",
        "second_command_works",
        "--timeout",
        "5000",
    ]);

    let snap_final = s.run_ok(&["snapshot"]);
    println!("=== Final snapshot ===\n{}", snap_final);
    assert!(
        snap_final.contains("second_command_works"),
        "Expected 'second_command_works' in final snapshot, got:\n{}",
        snap_final
    );

    // Clean up
    s.run_ok(&["type", "exit"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);
    sleep_ms(500);
}

/// Scenario 18: Multi-pane with real apps (bash + python)
///
/// Opens bash in the main pane, splits to add python3, and verifies that
/// both panes work independently: arithmetic in python, echo in bash.
#[test]
fn test_multi_pane_bash_and_python() {
    let s = Session::new();

    // Open bash in the main session and verify it's responsive
    open_bash_and_wait(&s, false);

    // Split to create second pane with python3
    let python3 = require_binary!("python3");
    s.run_ok(&["open", &python3, "--pane", "second"]);

    // Wait for python to start (python prompt goes to stdout, should be visible)
    sleep_ms(1000);

    // Discover pane indices
    let panes = list_pane_targets(&s.name);
    assert_eq!(
        panes.len(),
        2,
        "Expected 2 panes after split, got {:?}",
        panes
    );
    let first_pane = &panes[0]; // bash
    let second_pane = &panes[1]; // python

    println!(
        "Pane targets: first={} (bash), second={} (python)",
        first_pane, second_pane
    );

    // Snapshot the bash pane - should show our earlier BASH_READY output
    let snap_bash_initial = s.run_ok(&["snapshot", "--pane", first_pane]);
    println!("=== Bash pane (initial) ===\n{}", snap_bash_initial);
    assert!(
        snap_bash_initial.contains("BASH_READY"),
        "Bash pane should show earlier BASH_READY output, got:\n{}",
        snap_bash_initial
    );

    // Snapshot the python pane - should show >>> prompt
    let snap_python_initial = s.run_ok(&["snapshot", "--pane", second_pane]);
    println!("=== Python pane (initial) ===\n{}", snap_python_initial);
    assert!(
        snap_python_initial.contains(">>>"),
        "Python pane should show >>> prompt, got:\n{}",
        snap_python_initial
    );

    // Type arithmetic in the python pane
    s.run_ok(&["type", "--pane", second_pane, "2+2"]);
    sleep_ms(300);
    s.run_ok(&["send", "--pane", second_pane, "Enter"]);
    sleep_ms(500);

    // Snapshot the python pane - should show 4
    let snap_python_result = s.run_ok(&["snapshot", "--pane", second_pane]);
    println!("=== Python pane (after 2+2) ===\n{}", snap_python_result);
    assert!(
        snap_python_result.contains("4"),
        "Python pane should show result '4', got:\n{}",
        snap_python_result
    );

    // Type echo in the bash pane
    bash_exec_in_pane(&s, first_pane, "echo hi");
    sleep_ms(500);

    // Snapshot the bash pane - should show "hi"
    let snap_bash_result = s.run_ok(&["snapshot", "--pane", first_pane]);
    println!("=== Bash pane (after echo hi) ===\n{}", snap_bash_result);
    assert!(
        snap_bash_result.contains("hi"),
        "Bash pane should show 'hi', got:\n{}",
        snap_bash_result
    );

    // Verify panes are independent: python pane should NOT contain "echo hi"
    let snap_python_final = s.run_ok(&["snapshot", "--pane", second_pane]);
    assert!(
        !snap_python_final.contains("echo hi"),
        "Python pane should not contain bash commands, got:\n{}",
        snap_python_final
    );

    // Verify bash pane should NOT contain ">>>"
    let snap_bash_final = s.run_ok(&["snapshot", "--pane", first_pane]);
    assert!(
        !snap_bash_final.contains(">>>"),
        "Bash pane should not contain python prompt, got:\n{}",
        snap_bash_final
    );

    // Clean up: exit python
    s.run_ok(&["type", "--pane", second_pane, "exit()"]);
    sleep_ms(200);
    s.run_ok(&["send", "--pane", second_pane, "Enter"]);
    sleep_ms(500);

    // Clean up: exit bash
    s.run_ok(&["type", "--pane", first_pane, "exit"]);
    sleep_ms(200);
    s.run_ok(&["send", "--pane", first_pane, "Enter"]);
    sleep_ms(500);
}

/// Scenario 19: Long-running process monitoring
///
/// Starts a background job in bash and verifies that `status --json` reports
/// the session as alive while the background job runs, and that `jobs` shows
/// the background process.
#[test]
fn test_long_running_process_monitoring() {
    let s = Session::new();

    // Open bash and verify it's responsive
    open_bash_and_wait(&s, false);

    // Start a background sleep and echo confirmation
    s.run_ok(&["type", "sleep 10 & echo bg_started"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);

    // Wait for the confirmation message
    s.run_ok(&["wait", "--text", "bg_started", "--timeout", "5000"]);
    sleep_ms(500);

    let snap_bg = s.run_ok(&["snapshot"]);
    println!("=== Snapshot after background job start ===\n{}", snap_bg);
    assert!(
        snap_bg.contains("bg_started"),
        "Expected 'bg_started' in snapshot, got:\n{}",
        snap_bg
    );

    // Check status --json: session should be alive
    let status = s.run_ok(&["status", "--json"]);
    println!("=== Status JSON ===\n{}", status);
    let json: serde_json::Value = serde_json::from_str(&status).expect("invalid JSON");
    assert_eq!(
        json["alive"], true,
        "Session should be alive while background job runs"
    );

    // Run `jobs` to see the background process
    s.run_ok(&["type", "jobs"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);
    sleep_ms(500);

    // Snapshot to verify jobs output
    let snap_jobs = s.run_ok(&["snapshot"]);
    println!("=== Snapshot after jobs ===\n{}", snap_jobs);
    assert!(
        snap_jobs.contains("sleep"),
        "Expected 'sleep' in jobs output, got:\n{}",
        snap_jobs
    );

    // The key assertion: status works correctly during background job execution
    let status2 = s.run_ok(&["status", "--json"]);
    let json2: serde_json::Value = serde_json::from_str(&status2).expect("invalid JSON");
    assert_eq!(
        json2["alive"], true,
        "Session should remain alive with background job"
    );

    // Verify we can still interact with the shell while bg job runs
    s.run_ok(&["type", "echo shell_works"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);

    s.run_ok(&["wait", "--text", "shell_works", "--timeout", "5000"]);
    sleep_ms(500);

    let snap_final = s.run_ok(&["snapshot"]);
    println!("=== Final snapshot ===\n{}", snap_final);
    assert!(
        snap_final.contains("shell_works"),
        "Expected 'shell_works' in final snapshot, got:\n{}",
        snap_final
    );

    // Clean up: kill background job and exit
    s.run_ok(&["type", "kill %1 2>/dev/null; exit"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);
    sleep_ms(500);
}
