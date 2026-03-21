//! Color app — displays various ANSI color/style combinations.
//! Waits for 'q' to quit.

use std::io::{self, Write};

fn render() {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // Clear screen and home
    write!(out, "\x1b[2J\x1b[H").unwrap();
    // Line 1: Red text
    write!(out, "\x1b[31mRed Text\x1b[0m\r\n").unwrap();
    // Line 2: Green bold
    write!(out, "\x1b[1;32mGreen Bold\x1b[0m\r\n").unwrap();
    // Line 3: Blue underline
    write!(out, "\x1b[4;34mBlue Underline\x1b[0m\r\n").unwrap();
    // Line 4: Reverse video
    write!(out, "\x1b[7mReverse Video\x1b[0m\r\n").unwrap();
    // Line 5: Normal text
    write!(out, "Normal Text\r\n").unwrap();
    // Line 6: Yellow on blue background
    write!(out, "\x1b[33;44mYellow on Blue\x1b[0m\r\n").unwrap();
    out.flush().unwrap();
}

fn main() {
    // Enter raw mode
    let mut original: libc::termios = unsafe { std::mem::zeroed() };
    unsafe {
        libc::tcgetattr(libc::STDIN_FILENO, &mut original);
    }
    let mut raw = original;
    unsafe {
        libc::cfmakeraw(&mut raw);
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw);
    }

    render();

    // Wait for 'q'
    let mut buf = [0u8; 1];
    loop {
        let n = unsafe { libc::read(libc::STDIN_FILENO, buf.as_mut_ptr() as *mut _, 1) };
        if n == 1 && buf[0] == b'q' {
            break;
        }
    }

    // Restore terminal
    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original);
    }
    print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();
}
