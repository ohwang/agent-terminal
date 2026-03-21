//! Echo app — prints args, waits for 'q' to quit.
//! Used for lifecycle tests.

use std::io::{self, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Print READY marker
    print!("READY\r\n");
    // Print args joined by space
    print!("{}\r\n", args.join(" "));
    io::stdout().flush().unwrap();

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

    // Read single characters until 'q'
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
}
