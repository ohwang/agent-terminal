#[path = "common/mod.rs"]
mod common;
use common::Session;

use std::process::Command;

// ---------------------------------------------------------------------------
// 1. open --shell
// ---------------------------------------------------------------------------

#[test]
fn test_open_shell_keeps_session_alive_after_command_exits() {
    let s = Session::new();

    // Open a command that exits immediately, but with --shell to keep session alive
    s.run_ok(&["open", "echo done", "--shell"]);

    // Give the echo command time to exit and the shell to take over
    std::thread::sleep(std::time::Duration::from_millis(1500));

    // Session should still be alive because --shell wraps with `exec $SHELL`
    let out = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON from status");
    assert_eq!(
        json["alive"], true,
        "Session should still be alive after command exits with --shell"
    );

    // Snapshot should show the command output ("done") somewhere in scrollback
    let snap = s.run_ok(&["snapshot"]);
    assert!(
        snap.contains("done"),
        "Snapshot should contain the output of 'echo done', got:\n{}",
        snap
    );
}

#[test]
fn test_open_without_shell_session_dies_after_command_exits() {
    let s = Session::new();

    // Open a command that exits immediately WITHOUT --shell
    s.run_ok(&["open", "echo done"]);

    // Give the command time to exit
    std::thread::sleep(std::time::Duration::from_millis(1500));

    // Session should show the process as dead
    let out = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON from status");
    assert_eq!(
        json["alive"], false,
        "Session should be dead after command exits without --shell"
    );
}

// ---------------------------------------------------------------------------
// 2. open --no-stderr
// ---------------------------------------------------------------------------

#[test]
fn test_open_no_stderr_bash_prompt_visible() {
    let s = Session::new();

    // Open bash with --no-stderr so the prompt (which goes through stderr) is visible
    s.run_ok(&["open", "/bin/bash --norc --noprofile", "--no-stderr"]);

    // Wait for the bash prompt to appear (bash prints $ on stderr)
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let snap = s.run_ok(&["snapshot"]);

    // bash --norc --noprofile typically shows "bash-X.X$" or just "$"
    assert!(
        snap.contains("$") || snap.contains("bash"),
        "Snapshot should show a shell prompt with --no-stderr, got:\n{}",
        snap
    );
}

#[test]
fn test_open_without_no_stderr_prompt_not_visible() {
    let s = Session::new();

    // Open bash WITHOUT --no-stderr — stderr is captured to a file, so the
    // prompt (which bash prints via stderr) won't appear on screen.
    s.run_ok(&["open", "/bin/bash --norc --noprofile"]);

    std::thread::sleep(std::time::Duration::from_millis(1000));

    let snap = s.run_ok(&["snapshot"]);

    // Without --no-stderr, the bash prompt is redirected away. The snapshot
    // should be mostly empty (no "$" prompt visible).
    // Note: this is a best-effort check — the key point is the contrast with
    // the --no-stderr test above.
    let trimmed = snap.trim();
    assert!(
        !trimmed.contains("bash-") && !trimmed.contains("$ "),
        "Snapshot should NOT show a visible bash prompt without --no-stderr, got:\n{}",
        snap
    );
}

// ---------------------------------------------------------------------------
// 3. wait --exit
// ---------------------------------------------------------------------------

/// Helper: set `remain-on-exit on` for a tmux session so that #{pane_dead}
/// returns "1" after the process exits instead of tmux destroying the session.
fn set_remain_on_exit(session_name: &str) {
    let output = Command::new("tmux")
        .args(["set-option", "-t", session_name, "remain-on-exit", "on"])
        .output()
        .expect("failed to run tmux set-option");
    assert!(
        output.status.success(),
        "Failed to set remain-on-exit: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_wait_exit_succeeds_when_process_exits() {
    let s = Session::new();

    // Use the crash fixture which exits after ~500ms
    let crash_path = Session::fixture_path("crash");
    s.run_ok(&["open", &crash_path]);

    // Set remain-on-exit so tmux keeps the pane around after the process dies.
    // Without this, tmux destroys the session and #{pane_dead} can't be queried.
    set_remain_on_exit(&s.name);

    // wait --exit should succeed because the process exits within timeout
    let out = s.run_ok(&["wait", "--exit", "--timeout", "5000"]);
    assert!(
        out.contains("exited"),
        "wait --exit should report process exited, got: {}",
        out
    );
}

#[test]
fn test_wait_exit_timeout_when_process_still_running() {
    let s = Session::new();

    // Open a long-running fixture (counter runs forever until 'q')
    s.open_fixture_wait("counter", "Count:");

    // wait --exit with a short timeout should fail because the process is still running
    let out = s.run_fail(&["wait", "--exit", "--timeout", "500"]);
    assert!(
        out.contains("timed out"),
        "wait --exit should time out when process is still running, got: {}",
        out
    );
}

#[test]
fn test_wait_exit_with_short_lived_command() {
    let s = Session::new();

    // Open a command that prints output (so `open` returns quickly after first
    // render) and then sleeps before exiting. The `open` command waits up to 2s
    // for first output, so we need to print something immediately.
    s.run_ok(&["open", "echo waiting; sleep 2"]);

    // Set remain-on-exit so tmux keeps the pane around after the process dies,
    // allowing #{pane_dead} to return "1".
    set_remain_on_exit(&s.name);

    // wait --exit should succeed when the sleep finishes
    let out = s.run_ok(&["wait", "--exit", "--timeout", "5000"]);
    assert!(
        out.contains("exited"),
        "wait --exit should detect command exiting, got: {}",
        out
    );
}

// ---------------------------------------------------------------------------
// 4. Combined: --shell + --no-stderr for interactive bash testing
// ---------------------------------------------------------------------------

#[test]
fn test_combined_shell_and_no_stderr_interactive_bash() {
    let s = Session::new();

    // Open bash with both --shell and --no-stderr
    s.run_ok(&[
        "open",
        "/bin/bash --norc --noprofile",
        "--shell",
        "--no-stderr",
    ]);

    // Wait for the bash prompt to appear
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Type a command and press Enter
    s.run_ok(&["type", "echo test123"]);
    s.run_ok(&["send", "Enter"]);

    // Wait for the output to appear
    s.run_ok(&["wait", "--text", "test123", "--timeout", "3000"]);

    // Verify the output
    let snap = s.run_ok(&["snapshot"]);
    assert!(
        snap.contains("test123"),
        "Snapshot should contain 'test123' output, got:\n{}",
        snap
    );

    // Send exit to quit the inner bash command
    s.run_ok(&["type", "exit"]);
    s.run_ok(&["send", "Enter"]);

    // Give the shell time to exit and the outer shell to take over
    std::thread::sleep(std::time::Duration::from_millis(1500));

    // Session should still be alive because of --shell
    let status = s.run_ok(&["status", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&status).expect("invalid JSON from status");
    assert_eq!(
        json["alive"], true,
        "Session should still be alive after inner bash exits due to --shell"
    );
}
