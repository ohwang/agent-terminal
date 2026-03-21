//! Crash app — prints "Starting...", sleeps 500ms, prints "CRASHING" to stderr,
//! then exits with code 42. Tests process health detection.

use std::io::{self, Write};

fn main() {
    // Print to stdout
    print!("Starting...\n");
    io::stdout().flush().unwrap();

    // Sleep 500ms
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Print to stderr
    eprint!("CRASHING\n");
    io::stderr().flush().unwrap();

    // Exit with code 42
    std::process::exit(42);
}
