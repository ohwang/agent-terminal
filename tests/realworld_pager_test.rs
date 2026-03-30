#[path = "common/mod.rs"]
mod common;
use common::Session;

use std::fs;
use std::io::Write;

/// Scenario 8: less page navigation
/// Creates a temp file with 200 numbered lines, opens it in less,
/// tests navigation (j to scroll, G to end, g to top, q to quit).
#[test]
fn test_less_page_navigation() {
    let s = Session::new();

    // Create temp file with 200 numbered lines
    let tmp_path = format!("/tmp/agent-terminal-test-{}-lines.txt", std::process::id());
    {
        let mut f = fs::File::create(&tmp_path).expect("failed to create temp file");
        for i in 1..=200 {
            writeln!(f, "Line {}", i).expect("failed to write line");
        }
    }

    // Open a shell so the session persists after less exits
    s.run_ok(&["open", "bash", "--env", "TERM=xterm-256color"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Type the less command
    let less_cmd = format!("less {}\n", tmp_path);
    s.run_ok(&["type", &less_cmd]);

    // Wait for less to display the first line
    s.run_ok(&["wait", "--text", "Line 1", "--timeout", "5000"]);

    let snap_initial = s.run_ok(&["snapshot"]);
    eprintln!("=== LESS INITIAL SNAPSHOT ===\n{}", snap_initial);
    assert!(
        snap_initial.contains("Line 1"),
        "Initial snapshot should contain 'Line 1': {}",
        snap_initial
    );

    // Scroll down a few lines with j
    s.run_ok(&["send", "j"]);
    s.run_ok(&["send", "j"]);
    s.run_ok(&["send", "j"]);
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Go to end with G (shift+g)
    s.run_ok(&["send", "G"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    let snap_end = s.run_ok(&["snapshot"]);
    eprintln!("=== LESS END SNAPSHOT ===\n{}", snap_end);
    assert!(
        snap_end.contains("Line 200"),
        "End snapshot should contain 'Line 200': {}",
        snap_end
    );

    // Go back to top with g
    // In less, 'g' goes to the beginning of the file
    s.run_ok(&["send", "g"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    let snap_top = s.run_ok(&["snapshot"]);
    eprintln!("=== LESS TOP SNAPSHOT ===\n{}", snap_top);
    assert!(
        snap_top.contains("Line 1"),
        "Top snapshot should contain 'Line 1' after pressing g: {}",
        snap_top
    );

    // Quit less
    s.run_ok(&["send", "q"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // After quitting less, we should be back at the bash shell prompt
    // The alternate screen should be cleared (less uses alternate screen)
    let snap_after = s.run_ok(&["snapshot"]);
    eprintln!("=== AFTER LESS QUIT SNAPSHOT ===\n{}", snap_after);

    // Verify we are no longer in less
    let still_in_pager = snap_after.contains("(END)");
    assert!(
        !still_in_pager,
        "After quitting less, should not still show pager markers: {}",
        snap_after
    );

    // Clean up temp file
    let _ = fs::remove_file(&tmp_path);
}

/// Scenario 9: man page (alternate screen)
/// Opens `man ls`, verifies content, quits, and checks alternate screen is cleared.
#[test]
fn test_man_page_alternate_screen() {
    let s = Session::new();

    // Open a bash shell so the session persists after man/pager exits
    s.run_ok(&[
        "open",
        "bash",
        "--env",
        "TERM=xterm-256color",
        "--env",
        "PAGER=less",
        "--env",
        "MANPAGER=less",
    ]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Run man ls inside the shell
    s.run_ok(&["type", "man ls\n"]);

    // Wait for man page content to appear - look for common man page text
    // man ls should show "LS" or "NAME" section header
    // Try a generous timeout since man can be slow to format
    s.run_ok(&["wait", "--text", "LS", "--timeout", "8000"]);

    let snap_man = s.run_ok(&["snapshot"]);
    eprintln!("=== MAN PAGE SNAPSHOT ===\n{}", snap_man);

    // Verify man page content is showing
    let has_content = snap_man.contains("LS")
        || snap_man.contains("NAME")
        || snap_man.contains("ls")
        || snap_man.contains("list directory");
    assert!(
        has_content,
        "Man page snapshot should contain man page content: {}",
        snap_man
    );

    // Quit the man page pager
    s.run_ok(&["send", "q"]);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // After quitting, the alternate screen should be cleared
    // We should see a shell prompt since we opened bash first
    let snap_after = s.run_ok(&["snapshot"]);
    eprintln!("=== AFTER MAN QUIT SNAPSHOT ===\n{}", snap_after);

    // The man page was on the alternate screen; after quit, we should be back
    // at the bash shell. Verify pager markers are gone.
    let still_in_pager = snap_after.contains("(END)") || snap_after.contains("Manual page");
    assert!(
        !still_in_pager,
        "After quitting man, should not still show pager content: {}",
        snap_after
    );
}

/// Scenario 10: git log pager
/// Opens `git log --oneline -20` which should show commit history in a pager.
#[test]
fn test_git_log_pager() {
    let s = Session::new();

    // Create a temporary git repo with known history so the test is self-contained
    let tmp_repo = format!("/tmp/agent-terminal-git-test-{}", std::process::id());
    std::process::Command::new("bash")
        .args([
            "-c",
            &format!(
                "mkdir -p {0} && cd {0} && git init && \
                 git config user.email test@test && git config user.name test && \
                 echo a > file && git add . && git commit -m 'feat: initial commit' && \
                 echo b >> file && git add . && git commit -m 'fix: second commit' && \
                 echo c >> file && git add . && git commit -m 'docs: third commit'",
                tmp_repo
            ),
        ])
        .output()
        .expect("failed to create temp git repo");

    // Open a bash shell so the session persists after the pager exits
    s.run_ok(&[
        "open",
        "bash",
        "--env",
        "TERM=xterm-256color",
        "--env",
        "GIT_PAGER=less",
    ]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Run git log inside the shell
    let git_cmd = format!("git -C {} log --oneline\n", tmp_repo);
    s.run_ok(&["type", &git_cmd]);

    // Wait for known commit content to appear
    s.run_ok(&["wait", "--text", "feat", "--timeout", "8000"]);

    let snap_log = s.run_ok(&["snapshot"]);
    eprintln!("=== GIT LOG SNAPSHOT ===\n{}", snap_log);

    // Verify we see our known commit messages
    assert!(
        snap_log.contains("feat") && snap_log.contains("fix"),
        "Git log snapshot should contain known commit messages: {}",
        snap_log
    );

    // Quit the pager
    s.run_ok(&["send", "q"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // After quitting, we should be back at the bash shell prompt
    let snap_after = s.run_ok(&["snapshot"]);
    eprintln!("=== AFTER GIT LOG QUIT SNAPSHOT ===\n{}", snap_after);

    // Verify pager is no longer showing
    let still_in_pager = snap_after.contains("(END)");
    assert!(
        !still_in_pager,
        "After quitting git log pager, should not still show (END): {}",
        snap_after
    );

    // Clean up temp repo
    let _ = std::fs::remove_dir_all(&tmp_repo);
}
