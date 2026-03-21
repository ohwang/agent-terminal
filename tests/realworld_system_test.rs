#[path = "common/mod.rs"]
mod common;
use common::Session;

use std::fs;
use std::thread;
use std::time::Duration;

/// Helper: short sleep to let processes settle
fn pause(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

// ---------------------------------------------------------------------------
// Scenario 11: htop system monitor
// ---------------------------------------------------------------------------

#[test]
fn test_htop_system_monitor() {
    let s = Session::new();
    s.run_ok(&["open", "/opt/homebrew/bin/htop"]);

    // Wait for htop to load — look for common htop UI elements
    s.run_ok(&["wait", "--text", "PID", "--timeout", "8000"]);
    pause(500);

    // Snapshot should show process list with typical columns
    let snap = s.run_ok(&["snapshot"]);
    eprintln!("=== HTOP SNAPSHOT ===\n{}", snap);

    // htop should display CPU/Mem indicators and PID column
    let has_cpu = snap.contains("CPU") || snap.contains("cpu");
    let has_mem = snap.contains("Mem") || snap.contains("mem") || snap.contains("Mem");
    let has_pid = snap.contains("PID");
    assert!(
        has_cpu || has_mem || has_pid,
        "htop should show CPU, Mem, or PID. Got:\n{}",
        snap
    );

    // Send 'q' to quit htop
    s.run_ok(&["send", "q"]);
    pause(1500);

    // Verify process exits cleanly
    let status_out = s.run_ok(&["status", "--json"]);
    eprintln!("=== HTOP STATUS AFTER QUIT ===\n{}", status_out);
    let json: serde_json::Value = serde_json::from_str(&status_out).expect("invalid JSON");
    assert_eq!(
        json["alive"], false,
        "htop should have exited after pressing 'q'"
    );
}

// ---------------------------------------------------------------------------
// Scenario 12: grep colored output
// ---------------------------------------------------------------------------

#[test]
fn test_grep_colored_output() {
    let tmp_file = format!(
        "/tmp/agent-terminal-grep-test-{}.txt",
        std::process::id()
    );

    // Create a temp file with several lines
    fs::write(
        &tmp_file,
        "hello world\nfoo bar\nhello again\ngoodbye\n",
    )
    .expect("failed to create temp file");

    let s = Session::new();

    // Open bash so we have a persistent session, then run grep inside it
    s.run_ok(&["open", "/bin/bash --norc --noprofile"]);
    pause(1000);

    // Run grep with --color=always to produce ANSI color codes
    let grep_cmd = format!("grep --color=always hello {}", tmp_file);
    s.run_ok(&["type", &grep_cmd]);
    pause(200);
    s.run_ok(&["send", "Enter"]);

    // Wait for grep output to appear
    pause(1500);

    // Snapshot (plain) — should strip ANSI and show plain text
    let snap_plain = s.run_ok(&["snapshot"]);
    eprintln!("=== GREP PLAIN SNAPSHOT ===\n{}", snap_plain);

    // The matching lines should appear
    assert!(
        snap_plain.contains("hello world"),
        "grep output should contain 'hello world'. Got:\n{}",
        snap_plain
    );
    assert!(
        snap_plain.contains("hello again"),
        "grep output should contain 'hello again'. Got:\n{}",
        snap_plain
    );
    // Non-matching lines should NOT appear in the grep output lines
    // (they may appear in the temp file path, but not as standalone output lines)
    // Check that "foo bar" does not appear as a standalone line
    let has_foo_bar_line = snap_plain
        .lines()
        .any(|line| {
            let trimmed = line.trim();
            // Skip lines that are the command itself or the header
            !trimmed.contains("grep") && !trimmed.starts_with('[') && !trimmed.starts_with("─") && trimmed == "foo bar"
        });
    assert!(
        !has_foo_bar_line,
        "grep output should NOT contain 'foo bar' as a standalone output line. Got:\n{}",
        snap_plain
    );

    // Snapshot with --color to see ANSI color annotations
    let snap_color = s.run_ok(&["snapshot", "--color"]);
    eprintln!("=== GREP COLOR SNAPSHOT ===\n{}", snap_color);

    // The color snapshot should contain color annotations for the highlighted match
    // grep --color=always wraps matches in red/bold by default
    let has_color_annotation = snap_color.contains("[fg:")
        || snap_color.contains("[bold")
        || snap_color.contains("red");
    assert!(
        has_color_annotation,
        "Color snapshot should contain color annotations from grep. Got:\n{}",
        snap_color
    );

    // Exit bash
    s.run_ok(&["type", "exit"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(500);

    // Clean up temp file
    let _ = fs::remove_file(&tmp_file);
}

// ---------------------------------------------------------------------------
// Scenario 13: Unicode and emoji rendering
// ---------------------------------------------------------------------------

#[test]
fn test_unicode_and_emoji_rendering() {
    let s = Session::new();
    s.run_ok(&["open", "/bin/bash --norc --noprofile"]);

    // Wait for bash prompt — look for $ or bash prompt indicators
    pause(1000);
    // bash --norc gives a minimal prompt, usually "bash-X.X$ " or just "$ "
    // We'll just wait a moment and proceed since the prompt may vary
    let snap_initial = s.run_ok(&["snapshot"]);
    eprintln!("=== BASH INITIAL SNAPSHOT ===\n{}", snap_initial);

    // --- Test 1: Emoji and special characters ---
    // Use printf with octal/hex escapes to reliably produce UTF-8 bytes,
    // or use echo with actual UTF-8 characters typed directly
    s.run_ok(&["type", "echo 'Hello \u{1F30D} World \u{1F389} \u{2605} \u{2660} \u{2665} \u{2666} \u{2663}'"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(1000);

    let snap_emoji = s.run_ok(&["snapshot"]);
    eprintln!("=== EMOJI SNAPSHOT ===\n{}", snap_emoji);

    // Check for the text parts at minimum; emoji rendering may vary
    assert!(
        snap_emoji.contains("Hello"),
        "Snapshot should contain 'Hello'. Got:\n{}",
        snap_emoji
    );
    assert!(
        snap_emoji.contains("World"),
        "Snapshot should contain 'World'. Got:\n{}",
        snap_emoji
    );

    // Check for unicode special characters (BMP characters should render reliably)
    let has_star = snap_emoji.contains('\u{2605}'); // ★
    let has_spade = snap_emoji.contains('\u{2660}'); // ♠
    let has_heart = snap_emoji.contains('\u{2665}'); // ♥
    let has_diamond = snap_emoji.contains('\u{2666}'); // ♦
    let has_club = snap_emoji.contains('\u{2663}'); // ♣
    eprintln!(
        "Unicode chars found: star={} spade={} heart={} diamond={} club={}",
        has_star, has_spade, has_heart, has_diamond, has_club
    );

    // Check for emoji (non-BMP, may or may not render in tmux)
    let has_globe = snap_emoji.contains('\u{1F30D}'); // 🌍
    let has_party = snap_emoji.contains('\u{1F389}'); // 🎉
    eprintln!(
        "Emoji found: globe={} party={}",
        has_globe, has_party
    );

    // At least some unicode symbols should be captured
    let unicode_count = [has_star, has_spade, has_heart, has_diamond, has_club]
        .iter()
        .filter(|&&x| x)
        .count();
    if unicode_count == 0 {
        eprintln!("WARNING: No BMP unicode symbols captured in snapshot. This may indicate tmux is stripping them.");
    }
    // We assert at least the BMP symbols are captured (they're well-supported)
    assert!(
        unicode_count >= 3,
        "At least 3 of 5 BMP unicode symbols should be captured. Got {} of 5. Snapshot:\n{}",
        unicode_count,
        snap_emoji
    );

    // --- Test 2: CJK characters ---
    s.run_ok(&["type", "echo \"日本語テスト\""]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(1000);

    let snap_cjk = s.run_ok(&["snapshot"]);
    eprintln!("=== CJK SNAPSHOT ===\n{}", snap_cjk);

    // CJK characters should appear in the snapshot
    let has_cjk = snap_cjk.contains('日')
        || snap_cjk.contains('本')
        || snap_cjk.contains('語')
        || snap_cjk.contains("日本語");
    if has_cjk {
        eprintln!("CJK characters rendered correctly.");
    } else {
        eprintln!("WARNING: CJK characters not found in snapshot. tmux may not support them in this configuration.");
    }
    // Assert that at least some CJK text is captured
    assert!(
        has_cjk,
        "Snapshot should contain CJK characters (日本語テスト). Got:\n{}",
        snap_cjk
    );

    // --- Test 3: Box drawing characters ---
    // Use printf with $'...' syntax for the newlines, but actual UTF-8 box chars
    s.run_ok(&["type", "printf '\u{251C}\u{2500}\u{2500}\u{2524}\\n\u{2502}  \u{2502}\\n\u{2514}\u{2500}\u{2500}\u{2518}\\n'"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(1000);

    let snap_box = s.run_ok(&["snapshot"]);
    eprintln!("=== BOX DRAWING SNAPSHOT ===\n{}", snap_box);

    // Check for box drawing characters
    let has_box_top = snap_box.contains("├──┤") || snap_box.contains("├") || snap_box.contains("┤");
    let has_box_side = snap_box.contains('│');
    let has_box_bottom = snap_box.contains("└──┘") || snap_box.contains("└") || snap_box.contains("┘");
    eprintln!(
        "Box drawing found: top={} side={} bottom={}",
        has_box_top, has_box_side, has_box_bottom
    );

    assert!(
        has_box_top || has_box_side || has_box_bottom,
        "Snapshot should contain box drawing characters (├──┤, │, └──┘). Got:\n{}",
        snap_box
    );

    // Exit bash
    s.run_ok(&["type", "exit"]);
    pause(200);
    s.run_ok(&["send", "Enter"]);
    pause(500);
}
