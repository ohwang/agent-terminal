//! Slow app — full-screen TUI that auto-updates a frame counter every 100ms.
//! Tests FPS measurement. Reads input non-blocking; 'q' quits.

use std::io::{self, Write};

fn render(frame: u64) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "\x1b[2J\x1b[H").unwrap();
    write!(out, "Frame: {}\r\n", frame).unwrap();
    write!(out, "[q] quit\r\n").unwrap();
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
        // Set VMIN=0, VTIME=1 for non-blocking read with 100ms timeout
        raw.c_cc[libc::VMIN] = 0;
        raw.c_cc[libc::VTIME] = 1; // 1 decisecond = 100ms
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw);
    }

    let mut frame: u64 = 0;
    render(frame);

    let mut buf = [0u8; 1];
    loop {
        let n = unsafe { libc::read(libc::STDIN_FILENO, buf.as_mut_ptr() as *mut _, 1) };
        if n == 1 && buf[0] == b'q' {
            break;
        }
        // Whether we read 0 bytes (timeout) or got a non-q key, render next frame
        frame += 1;
        render(frame);
    }

    // Restore terminal
    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original);
    }
    print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();
}
