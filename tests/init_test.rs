#[path = "common/mod.rs"]
mod common;

use std::fs;

#[test]
fn test_init_detects_ratatui() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "my-tui"
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
"#,
    )
    .unwrap();

    let bin = common::Session::bin_path();
    let output = std::process::Command::new(&bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("failed to run init");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "init should succeed: {}", stdout);
    assert!(
        stdout.contains("ratatui") || stdout.contains("crossterm"),
        "Should detect ratatui/crossterm: {}",
        stdout
    );

    // Verify test file was created
    let test_file = dir.path().join("tests/tui/basic_test.sh");
    assert!(test_file.exists(), "Should create tests/tui/basic_test.sh");

    let content = fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("agent-terminal"), "Generated test should reference agent-terminal");
}

#[test]
fn test_init_detects_bubbletea() {
    let dir = tempfile::tempdir().unwrap();
    let go_mod = dir.path().join("go.mod");
    fs::write(
        &go_mod,
        r#"module my-tui
go 1.21
require github.com/charmbracelet/bubbletea v0.25.0
"#,
    )
    .unwrap();

    let bin = common::Session::bin_path();
    let output = std::process::Command::new(&bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("failed to run init");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "init should succeed: {}", stdout);
    assert!(
        stdout.contains("bubbletea"),
        "Should detect bubbletea: {}",
        stdout
    );
}

#[test]
fn test_init_detects_textual() {
    let dir = tempfile::tempdir().unwrap();
    let req = dir.path().join("requirements.txt");
    fs::write(&req, "textual>=0.40\nrich\n").unwrap();

    let bin = common::Session::bin_path();
    let output = std::process::Command::new(&bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("failed to run init");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "init should succeed: {}", stdout);
    assert!(
        stdout.contains("textual") || stdout.contains("rich"),
        "Should detect textual/rich: {}",
        stdout
    );
}

#[test]
fn test_init_no_framework() {
    let dir = tempfile::tempdir().unwrap();
    // Empty directory, no framework files

    let bin = common::Session::bin_path();
    let output = std::process::Command::new(&bin)
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("failed to run init");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should still succeed and generate a generic test
    assert!(output.status.success(), "init should succeed even without framework: stderr={}",
            String::from_utf8_lossy(&output.stderr));
}
