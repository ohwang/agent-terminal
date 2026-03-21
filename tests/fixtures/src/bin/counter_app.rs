//! Counter app — j increments, k decrements, q quits.
//! Full-screen raw mode TUI with colored output.

use std::io::{self, Write};

fn render(count: i64) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // Clear screen and home
    write!(out, "\x1b[2J\x1b[H").unwrap();
    // "Count: " in normal, then number in green bold
    write!(out, "Count: \x1b[1;32m{}\x1b[0m\r\n", count).unwrap();
    write!(out, "[j] +1  [k] -1  [q] quit\r\n").unwrap();
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

    let mut count: i64 = 0;
    render(count);

    let mut buf = [0u8; 1];
    loop {
        let n = unsafe { libc::read(libc::STDIN_FILENO, buf.as_mut_ptr() as *mut _, 1) };
        if n == 1 {
            match buf[0] {
                b'j' => {
                    count += 1;
                    render(count);
                }
                b'k' => {
                    count -= 1;
                    render(count);
                }
                b'q' => break,
                _ => {}
            }
        }
    }

    // Restore terminal
    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original);
    }
    // Clear screen on exit
    print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();
}
