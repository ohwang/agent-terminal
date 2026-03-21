#[path = "common/mod.rs"]
mod common;
use common::Session;

use std::thread;
use std::time::Duration;

/// Helper: short sleep to let the terminal process keystrokes
fn pause(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

/// Helper: open bash with an explicit PS1 prompt so we can reliably detect it.
/// bash --norc --noprofile may produce no visible prompt on some systems.
fn open_bash_with_prompt(s: &Session) {
    s.run_ok(&["open", "/bin/bash --norc --noprofile"]);
    // Give bash a moment to start
    pause(1000);
    // Set an explicit prompt so we can detect it reliably
    s.run_ok(&["type", "export PS1='READY$ '"]);
    pause(100);
    s.run_ok(&["send", "Enter"]);
    s.run_ok(&["wait", "--text", "READY$", "--timeout", "5000"]);
    pause(300);
}

/// Scenario 20: Nested alternate screen (nvim help)
///
/// nvim runs on an alternate screen. Opening :help creates a split within
/// that alternate screen. This tests that agent-terminal can snapshot and
/// interact through nested alternate-screen layers.
#[test]
fn test_nested_altscreen_nvim_help() {
    let s = Session::new();
    s.run_ok(&["open", "/opt/homebrew/bin/nvim"]);

    // Wait for nvim to load. This system may have LazyVim configured, which
    // shows a splash screen instead of bare ~ lines. We wait for either.
    // LazyVim splash shows "Find File", plain nvim shows "~".
    pause(2000);
    s.run_ok(&["wait", "--regex", r"~|NVIM|Find File|LazyVim", "--timeout", "8000"]);
    pause(500);

    // Snapshot and verify we are in nvim
    let snap_initial = s.run_ok(&["snapshot"]);
    eprintln!("=== SCENARIO 20: INITIAL NVIM ===\n{}", snap_initial);
    let in_nvim = snap_initial.contains('~')
        || snap_initial.contains("NVIM")
        || snap_initial.contains("Find File")
        || snap_initial.contains("LazyVim");
    assert!(
        in_nvim,
        "Initial snapshot should show nvim (~ lines, NVIM, or LazyVim splash)"
    );

    // Open help: type :help then send Enter
    s.run_ok(&["type", ":help"]);
    pause(300);
    s.run_ok(&["send", "Enter"]);
    pause(1000);

    // Wait for help content to appear
    s.run_ok(&["wait", "--regex", r"help\.txt|VIM -|NVIM|quickref", "--timeout", "8000"]);

    // Snapshot — should show help in a split
    let snap_help = s.run_ok(&["snapshot"]);
    eprintln!("=== SCENARIO 20: HELP SCREEN ===\n{}", snap_help);
    let has_help = snap_help.contains("help.txt")
        || snap_help.contains("VIM")
        || snap_help.contains("NVIM")
        || snap_help.contains("quickref");
    assert!(
        has_help,
        "Help snapshot should contain help-related content"
    );

    // Close help: type :q then Enter
    s.run_ok(&["type", ":q"]);
    pause(300);
    s.run_ok(&["send", "Enter"]);
    pause(500);

    // Snapshot — should be back to normal nvim
    let snap_after = s.run_ok(&["snapshot"]);
    eprintln!("=== SCENARIO 20: AFTER CLOSING HELP ===\n{}", snap_after);

    // After closing help we should see either ~ lines (empty buffer) or the
    // LazyVim dashboard again. The help split should be gone.
    let back_to_normal = snap_after.contains('~')
        || snap_after.contains("Find File")
        || snap_after.contains("LazyVim");
    assert!(
        back_to_normal,
        "After closing help, should be back to normal nvim. Got:\n{}",
        snap_after
    );

    // Quit nvim
    s.run_ok(&["type", ":qa!"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(500);
}

/// Bonus Scenario A: snapshot --diff mode
///
/// Tests that consecutive --diff snapshots detect changes in terminal content.
#[test]
fn test_snapshot_diff_mode() {
    let s = Session::new();
    open_bash_with_prompt(&s);

    // Take first snapshot to establish baseline for diff
    let snap_baseline = s.run_ok(&["snapshot", "--diff"]);
    eprintln!("=== BONUS A: BASELINE DIFF ===\n{}", snap_baseline);

    // Type a command and execute it
    s.run_ok(&["type", "echo \"change1\""]);
    pause(100);
    s.run_ok(&["send", "Enter"]);

    // Wait for the output to appear
    s.run_ok(&["wait", "--text", "change1", "--timeout", "5000"]);
    pause(300);

    // Take a diff snapshot — should show what changed
    let diff1 = s.run_ok(&["snapshot", "--diff"]);
    eprintln!("=== BONUS A: DIFF AFTER change1 ===\n{}", diff1);

    // The diff should NOT say "(no changes)" since we typed a command
    assert!(
        !diff1.contains("(no changes)"),
        "Diff should detect changes after echo change1. Got:\n{}",
        diff1
    );
    // The diff should include the new content
    assert!(
        diff1.contains("change1"),
        "Diff should show 'change1' in the output. Got:\n{}",
        diff1
    );

    // Now make another change
    s.run_ok(&["type", "echo \"change2\""]);
    pause(100);
    s.run_ok(&["send", "Enter"]);

    // Wait for change2 to appear
    s.run_ok(&["wait", "--text", "change2", "--timeout", "5000"]);
    pause(300);

    // Take another diff snapshot
    let diff2 = s.run_ok(&["snapshot", "--diff"]);
    eprintln!("=== BONUS A: DIFF AFTER change2 ===\n{}", diff2);

    // Should show the new change
    assert!(
        diff2.contains("change2"),
        "Second diff should show 'change2'. Got:\n{}",
        diff2
    );
}

/// Bonus Scenario B: wait with --stable and --regex
///
/// Tests --stable (waits until screen stops changing) and --regex (pattern match).
#[test]
fn test_wait_stable_and_regex() {
    let s = Session::new();
    open_bash_with_prompt(&s);

    // Test --stable: screen should already be stable (prompt is showing, nothing changing)
    let start = std::time::Instant::now();
    let stable_out = s.run_ok(&["wait", "--stable", "500", "--timeout", "5000"]);
    let stable_elapsed = start.elapsed().as_millis();
    eprintln!(
        "=== BONUS B: STABLE WAIT (idle screen) took {}ms ===\n{}",
        stable_elapsed, stable_out
    );
    // Should succeed — the screen is not changing
    assert!(
        !stable_out.is_empty(),
        "Stable wait on idle screen should return a snapshot"
    );

    // Now generate output that takes time (items with delays)
    s.run_ok(&["type", "for i in 1 2 3; do echo \"item$i\"; sleep 0.3; done"]);
    pause(100);
    s.run_ok(&["send", "Enter"]);

    // Wait for all items to finish outputting, then screen should stabilize
    // The loop takes ~0.9s, so we wait for the last item then check stable
    s.run_ok(&["wait", "--text", "item3", "--timeout", "5000"]);

    // After all output is done, --stable should succeed once the screen settles
    let stable_out2 = s.run_ok(&["wait", "--stable", "1000", "--timeout", "8000"]);
    eprintln!(
        "=== BONUS B: STABLE WAIT (after loop output) ===\n{}",
        stable_out2
    );
    // Verify all items are present
    assert!(
        stable_out2.contains("item1")
            && stable_out2.contains("item2")
            && stable_out2.contains("item3"),
        "After stable wait, all items should be visible"
    );

    // Test --regex: echo a specific pattern and match with regex
    s.run_ok(&["type", "echo \"code:42\""]);
    pause(100);
    s.run_ok(&["send", "Enter"]);
    pause(300);

    let regex_out = s.run_ok(&["wait", "--regex", r"code:\d+", "--timeout", "5000"]);
    eprintln!("=== BONUS B: REGEX WAIT ===\n{}", regex_out);
    assert!(
        regex_out.contains("code:42"),
        "Regex wait should match 'code:42'. Got:\n{}",
        regex_out
    );
}
