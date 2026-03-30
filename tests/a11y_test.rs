#[path = "common/mod.rs"]
mod common;
use common::Session;

#[test]
fn test_a11y_check_runs() {
    let counter = Session::fixture_path("counter");

    let bin = Session::bin_path();
    let output = std::process::Command::new(&bin)
        .args(["a11y-check", &counter])
        .output()
        .expect("failed to run a11y-check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should run all checks and report results
    assert!(
        combined.contains("NO_COLOR")
            || combined.contains("no.color")
            || combined.contains("nocolor"),
        "Should check NO_COLOR: {}",
        combined
    );
    assert!(
        combined.contains("TERM=dumb") || combined.contains("dumb"),
        "Should check TERM=dumb: {}",
        combined
    );
    assert!(
        combined.contains("resize") || combined.contains("Resize"),
        "Should check resize: {}",
        combined
    );
}

#[test]
fn test_a11y_check_resize_survives() {
    let counter = Session::fixture_path("counter");

    let bin = Session::bin_path();
    let output = std::process::Command::new(&bin)
        .args(["a11y-check", &counter])
        .output()
        .expect("failed to run a11y-check");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The resize check output looks like: "resize handling ............ at-a11y-resize\n✓"
    // or on the same line. Check that the section doesn't contain ✗
    let resize_section: String = stdout
        .lines()
        .skip_while(|l| !l.to_lowercase().contains("resize"))
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        !resize_section.contains("✗") && !resize_section.is_empty(),
        "Resize check should not fail for counter fixture. Section: '{}'",
        resize_section
    );
}

#[test]
fn test_a11y_check_term_dumb_survives() {
    let counter = Session::fixture_path("counter");

    let bin = Session::bin_path();
    let output = std::process::Command::new(&bin)
        .args(["a11y-check", &counter])
        .output()
        .expect("failed to run a11y-check");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that the TERM=dumb section doesn't contain ✗
    let dumb_section: String = stdout
        .lines()
        .skip_while(|l| !l.to_lowercase().contains("dumb"))
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        !dumb_section.contains("✗") && !dumb_section.is_empty(),
        "TERM=dumb check should not fail for counter fixture. Section: '{}'",
        dumb_section
    );
}
