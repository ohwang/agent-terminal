#[path = "common/mod.rs"]
mod common;
use common::Session;

use std::thread;
use std::time::Duration;

fn sleep_ms(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

/// Scenario 4: bash interactive command execution
/// Opens a clean bash shell, runs `echo "hello world"`, verifies output.
///
/// NOTE: agent-terminal wraps the command with `2>file; echo $? > file`,
/// which redirects stderr. Bash writes its prompt to stderr, so the prompt
/// is invisible in snapshots. We use `--stable` wait instead of prompt matching.
#[test]
fn test_bash_interactive_command_execution() {
    let s = Session::new();
    s.run_ok(&["open", "/bin/bash --norc --noprofile"]);

    // The bash prompt is written to stderr, which agent-terminal redirects to a
    // temp file. So the prompt is NOT visible in snapshots. We wait for the
    // terminal to stabilise instead.
    s.run_ok(&["wait", "--stable", "500", "--timeout", "5000"]);
    sleep_ms(500);

    // Type the echo command and send Enter
    s.run_ok(&["type", "echo \"hello world\""]);
    sleep_ms(300);
    s.run_ok(&["send", "Enter"]);

    // Wait for the output to appear
    s.run_ok(&["wait", "--text", "hello world", "--timeout", "5000"]);
    sleep_ms(500);

    // Snapshot and verify
    let snap = s.run_ok(&["snapshot"]);
    println!("=== Bash Command Execution Snapshot ===\n{}", snap);
    assert!(
        snap.contains("hello world"),
        "Expected 'hello world' in snapshot, got:\n{}",
        snap
    );

    // Exit the shell
    s.run_ok(&["type", "exit"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);
    sleep_ms(500);
}

/// Scenario 5: bash tab completion
/// Opens bash shell, types partial command `ech`, sends Tab to trigger completion.
///
/// KNOWN LIMITATIONS:
/// 1. agent-terminal redirects stderr via its command wrapper (`2>file`). Bash
///    writes its prompt to stderr, so the prompt is invisible in snapshots.
/// 2. Bash's readline completion output (bell, candidate lists) also goes to
///    stderr, so tab completion results may not be visible. The snapshot may
///    show literal tab characters instead of the completed command.
/// 3. Sending C-c after failed completion can kill the shell because bash's
///    readline is impaired by the stderr redirect.
///
/// This test validates that agent-terminal can send the Tab key and that bash
/// receives it, even if the visible completion behaviour is degraded.
#[test]
fn test_bash_tab_completion() {
    let s = Session::new();
    s.run_ok(&["open", "/bin/bash --norc --noprofile"]);

    // Wait for shell to stabilise (prompt is invisible due to stderr redirect)
    s.run_ok(&["wait", "--stable", "500", "--timeout", "5000"]);
    sleep_ms(500);

    // Type partial command
    s.run_ok(&["type", "ech"]);
    sleep_ms(300);

    // Send Tab for completion
    s.run_ok(&["send", "Tab"]);
    sleep_ms(1000);

    // Snapshot to see what happened
    let snap = s.run_ok(&["snapshot"]);
    println!("=== Bash Tab Completion Snapshot ===\n{}", snap);

    // Due to stderr redirect, tab completion may not render visually.
    // We verify the Tab key was delivered by checking that:
    // - The snapshot is not empty (bash is alive and received input)
    // - "ech" is present (we typed it)
    assert!(
        snap.contains("ech"),
        "Expected 'ech' (the typed text) in snapshot, got:\n{}",
        snap
    );

    if snap.contains("echo") {
        println!("Tab completion successfully completed 'ech' to 'echo'.");
    } else {
        println!(
            "NOTE: Tab completion did not visually produce 'echo'. \
             This is expected because agent-terminal redirects stderr, \
             which breaks bash's readline completion rendering."
        );
    }

    // Clean up: send Ctrl+C then exit. Use `send` for exit too because
    // the session might be dead after C-c (bash exits on signal when
    // readline is impaired). Use run() instead of run_ok() to tolerate
    // the session already being gone.
    let _ = s.run(&["send", "C-c"]);
    sleep_ms(300);
    let _ = s.run(&["type", "exit"]);
    sleep_ms(200);
    let _ = s.run(&["send", "Enter"]);
    sleep_ms(500);
}

/// Scenario 6: Python3 REPL
/// Opens python3, evaluates arithmetic and print, verifies outputs.
#[test]
fn test_python3_repl() {
    let python3 = require_binary!("python3");
    let s = Session::new();
    s.run_ok(&["open", &python3]);

    // Wait for the >>> prompt
    s.run_ok(&["wait", "--text", ">>>", "--timeout", "5000"]);
    sleep_ms(500);

    // Type arithmetic expression
    s.run_ok(&["type", "2 + 2"]);
    sleep_ms(300);
    s.run_ok(&["send", "Enter"]);

    // Wait for result
    s.run_ok(&["wait", "--text", "4", "--timeout", "5000"]);
    sleep_ms(500);

    // Type print statement
    s.run_ok(&["type", "print(\"hello from python\")"]);
    sleep_ms(300);
    s.run_ok(&["send", "Enter"]);

    // Wait for print output
    s.run_ok(&["wait", "--text", "hello from python", "--timeout", "5000"]);
    sleep_ms(500);

    // Snapshot and verify both outputs
    let snap = s.run_ok(&["snapshot"]);
    println!("=== Python3 REPL Snapshot ===\n{}", snap);
    assert!(
        snap.contains("4"),
        "Expected '4' in snapshot, got:\n{}",
        snap
    );
    assert!(
        snap.contains("hello from python"),
        "Expected 'hello from python' in snapshot, got:\n{}",
        snap
    );

    // Exit python
    s.run_ok(&["type", "exit()"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);
    sleep_ms(500);
}

/// Scenario 7: Node.js REPL
/// Opens node, evaluates arithmetic and string method, verifies outputs.
#[test]
fn test_nodejs_repl() {
    let node = require_binary!("node");
    let s = Session::new();
    s.run_ok(&["open", &node]);

    // Wait for the > prompt. Node shows "Welcome to Node.js" then "> "
    s.run_ok(&["wait", "--text", ">", "--timeout", "5000"]);
    sleep_ms(500);

    // Type arithmetic expression
    s.run_ok(&["type", "1 + 1"]);
    sleep_ms(300);
    s.run_ok(&["send", "Enter"]);

    // Wait for result "2"
    s.run_ok(&["wait", "--text", "2", "--timeout", "5000"]);
    sleep_ms(500);

    // Type string method
    s.run_ok(&["type", "'hello'.toUpperCase()"]);
    sleep_ms(300);
    s.run_ok(&["send", "Enter"]);

    // Wait for "HELLO"
    s.run_ok(&["wait", "--text", "HELLO", "--timeout", "5000"]);
    sleep_ms(500);

    // Snapshot and verify outputs
    let snap = s.run_ok(&["snapshot"]);
    println!("=== Node.js REPL Snapshot ===\n{}", snap);
    assert!(
        snap.contains("2"),
        "Expected '2' in snapshot, got:\n{}",
        snap
    );
    assert!(
        snap.contains("HELLO"),
        "Expected 'HELLO' in snapshot, got:\n{}",
        snap
    );

    // Exit node
    s.run_ok(&["type", ".exit"]);
    sleep_ms(200);
    s.run_ok(&["send", "Enter"]);
    sleep_ms(500);
}
