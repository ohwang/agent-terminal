//! Resize app — shows terminal dimensions, handles SIGWINCH to update.
//! Displays "Size: COLSxROWS". Waits for 'q' to quit. Raw mode.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

static RESIZED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigwinch(_sig: libc::c_int) {
    RESIZED.store(true, Ordering::SeqCst);
}

fn get_terminal_size() -> (u16, u16) {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) };
    if ret == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
        (ws.ws_col, ws.ws_row)
    } else {
        (80, 24) // fallback
    }
}

fn render(cols: u16, rows: u16) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "\x1b[2J\x1b[H").unwrap();
    write!(out, "Size: {}x{}\r\n", cols, rows).unwrap();
    write!(out, "[q] quit\r\n").unwrap();
    out.flush().unwrap();
}

fn main() {
    // Install SIGWINCH handler
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handle_sigwinch as *const () as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGWINCH, &sa, std::ptr::null_mut());
    }

    // Enter raw mode
    let mut original: libc::termios = unsafe { std::mem::zeroed() };
    unsafe {
        libc::tcgetattr(libc::STDIN_FILENO, &mut original);
    }
    let mut raw = original;
    unsafe {
        libc::cfmakeraw(&mut raw);
        // Use a short timeout so we can check the RESIZED flag periodically
        raw.c_cc[libc::VMIN] = 0;
        raw.c_cc[libc::VTIME] = 2; // 200ms
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw);
    }

    let (cols, rows) = get_terminal_size();
    render(cols, rows);

    let mut buf = [0u8; 1];
    loop {
        let n = unsafe { libc::read(libc::STDIN_FILENO, buf.as_mut_ptr() as *mut _, 1) };
        if n == 1 && buf[0] == b'q' {
            break;
        }

        // Check if we received SIGWINCH
        if RESIZED.swap(false, Ordering::SeqCst) {
            let (cols, rows) = get_terminal_size();
            render(cols, rows);
        }
    }

    // Restore terminal
    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original);
    }
    print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();
}
