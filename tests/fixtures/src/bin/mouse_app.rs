//! Mouse app — enables SGR mouse tracking, prints click coordinates.
//! Shows "Click anywhere" initially. On click, shows "Clicked: row=R col=C".
//! Waits for 'q' to quit.

use std::io::{self, Write};

fn render(msg: &str) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "\x1b[2J\x1b[H").unwrap();
    write!(out, "{}\r\n", msg).unwrap();
    write!(out, "[q] quit\r\n").unwrap();
    out.flush().unwrap();
}

/// Parse SGR mouse sequence: \x1b[<Btn;Col;Row[Mm]
/// Returns Some((row, col)) on button press (trailing 'M'), None otherwise.
fn parse_sgr_mouse(buf: &[u8], len: usize) -> Option<(u32, u32)> {
    // Minimum: \x1b [ < N ; N ; N M = 9 bytes
    if len < 9 {
        return None;
    }
    // Must start with \x1b[<
    if buf[0] != 0x1b || buf[1] != b'[' || buf[2] != b'<' {
        return None;
    }
    // Must end with 'M' (press) not 'm' (release)
    let terminator = buf[len - 1];
    if terminator != b'M' {
        return None;
    }
    // Parse the three semicolon-separated numbers between '<' and 'M'
    let payload = std::str::from_utf8(&buf[3..len - 1]).ok()?;
    let parts: Vec<&str> = payload.split(';').collect();
    if parts.len() != 3 {
        return None;
    }
    let _btn: u32 = parts[0].parse().ok()?;
    let col: u32 = parts[1].parse().ok()?;
    let row: u32 = parts[2].parse().ok()?;
    Some((row, col))
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

    // Enable mouse tracking: X10 basic + SGR extended
    print!("\x1b[?1000h\x1b[?1006h");
    io::stdout().flush().unwrap();

    render("Click anywhere");

    let mut buf = [0u8; 64];
    loop {
        let n = unsafe { libc::read(libc::STDIN_FILENO, buf.as_mut_ptr() as *mut _, buf.len()) };
        if n <= 0 {
            continue;
        }
        let n = n as usize;

        // Check for 'q' (single byte, not part of an escape sequence)
        if n == 1 && buf[0] == b'q' {
            break;
        }

        // Try to parse SGR mouse event
        if let Some((row, col)) = parse_sgr_mouse(&buf, n) {
            let msg = format!("Clicked: row={} col={}", row, col);
            render(&msg);
        }
    }

    // Disable mouse tracking
    print!("\x1b[?1006l\x1b[?1000l");
    io::stdout().flush().unwrap();

    // Restore terminal
    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original);
    }
    print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();
}
