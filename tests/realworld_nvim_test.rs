#[path = "common/mod.rs"]
mod common;
use common::Session;

use std::fs;
use std::thread;
use std::time::Duration;

/// We use `nvim --clean` to avoid user configuration (LazyVim, etc.)
/// and get predictable vanilla nvim behavior.
const NVIM: &str = "/opt/homebrew/bin/nvim --clean";

/// Helper: short sleep to let nvim process keystrokes
fn pause(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

#[test]
fn test_nvim_edit_save_quit() {
    let tmp_file = format!("/tmp/agent-terminal-nvim-test-{}.txt", std::process::id());

    // Create an empty temp file
    fs::write(&tmp_file, "").expect("failed to create temp file");

    let s = Session::new();
    let cmd = format!("{} {}", NVIM, tmp_file);
    s.run_ok(&["open", &cmd]);

    // Wait for nvim to load — vanilla nvim shows ~ on empty lines
    s.run_ok(&["wait", "--text", "~", "--timeout", "8000"]);
    pause(300);

    // Take a snapshot to see what nvim looks like after loading
    let snap_initial = s.run_ok(&["snapshot"]);
    eprintln!("=== INITIAL NVIM SNAPSHOT ===\n{}", snap_initial);

    // Enter insert mode
    s.run_ok(&["send", "i"]);
    pause(200);

    // Type some text
    s.run_ok(&["type", "Hello from agent-terminal"]);
    pause(300);

    // Verify the typed text appears
    s.run_ok(&[
        "wait",
        "--text",
        "Hello from agent-terminal",
        "--timeout",
        "5000",
    ]);

    let snap_after_type = s.run_ok(&["snapshot"]);
    eprintln!("=== AFTER TYPING SNAPSHOT ===\n{}", snap_after_type);

    // Press Escape to return to normal mode
    s.run_ok(&["send", "Escape"]);
    pause(200);

    // Save and quit with :wq
    s.run_ok(&["type", ":wq"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(500);

    // Wait for nvim to exit — the process should die and we should see a shell or exit
    // Give it a moment to write and exit
    thread::sleep(Duration::from_secs(2));

    // Verify the file was written with the correct contents
    let contents = fs::read_to_string(&tmp_file).expect("failed to read temp file");
    eprintln!("=== FILE CONTENTS ===\n{:?}", contents);
    assert!(
        contents.contains("Hello from agent-terminal"),
        "File should contain typed text, got: {:?}",
        contents
    );

    // Clean up
    let _ = fs::remove_file(&tmp_file);
}

#[test]
fn test_nvim_window_splits() {
    let s = Session::new();
    s.run_ok(&["open", NVIM]);

    // Wait for nvim to load — vanilla nvim shows ~ on empty lines
    s.run_ok(&["wait", "--text", "~", "--timeout", "8000"]);
    pause(500);

    // Create a vertical split
    s.run_ok(&["type", ":vsplit"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(500);

    // Take a snapshot — should show the split divider
    let snap_split = s.run_ok(&["snapshot"]);
    eprintln!("=== VSPLIT SNAPSHOT ===\n{}", snap_split);

    // The split should show a vertical divider (│) or at least two sets of ~ lines
    // nvim uses │ for window separators
    let has_divider = snap_split.contains('│') || snap_split.contains('|');
    assert!(has_divider, "Snapshot should show a vertical split divider");

    // Navigate between windows: Ctrl+W then l (move right)
    s.run_ok(&["send", "C-w"]);
    pause(100);
    s.run_ok(&["send", "l"]);
    pause(300);

    // Navigate back: Ctrl+W then h (move left)
    s.run_ok(&["send", "C-w"]);
    pause(100);
    s.run_ok(&["send", "h"]);
    pause(300);

    let snap_after_nav = s.run_ok(&["snapshot"]);
    eprintln!("=== AFTER NAVIGATION SNAPSHOT ===\n{}", snap_after_nav);

    // Still should have the split
    let still_split = snap_after_nav.contains('│') || snap_after_nav.contains('|');
    assert!(
        still_split,
        "Split should still be visible after navigation"
    );

    // Quit all
    s.run_ok(&["type", ":qa!"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(500);
}

#[test]
fn test_nvim_help_screen() {
    let s = Session::new();
    s.run_ok(&["open", NVIM]);

    // Wait for nvim to load
    s.run_ok(&["wait", "--text", "~", "--timeout", "8000"]);
    pause(500);

    // Take initial snapshot
    let snap_initial = s.run_ok(&["snapshot"]);
    eprintln!("=== INITIAL NVIM SNAPSHOT ===\n{}", snap_initial);

    // Open help
    s.run_ok(&["type", ":help"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(500);

    // Wait for help content to appear
    s.run_ok(&["wait", "--text", "help.txt", "--timeout", "8000"]);

    let snap_help = s.run_ok(&["snapshot"]);
    eprintln!("=== HELP SCREEN SNAPSHOT ===\n{}", snap_help);

    // Help should contain typical help text
    let has_help_content =
        snap_help.contains("help.txt") || snap_help.contains("VIM") || snap_help.contains("NVIM");
    assert!(has_help_content, "Help screen should show help content");

    // Close help window
    s.run_ok(&["type", ":q"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(500);

    // Should be back to normal nvim
    let snap_after_close = s.run_ok(&["snapshot"]);
    eprintln!("=== AFTER CLOSING HELP SNAPSHOT ===\n{}", snap_after_close);

    // The help text should be gone, back to normal nvim with ~ lines
    assert!(
        snap_after_close.contains('~'),
        "After closing help, should see normal nvim with ~ lines"
    );

    // Quit
    s.run_ok(&["type", ":qa!"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(500);
}
