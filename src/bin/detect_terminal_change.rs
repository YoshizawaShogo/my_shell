use libc::{SIGINT, SIGWINCH};
use std::fs::File;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

#[repr(C)]
#[derive(Debug)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
}

const TIOCGWINSZ: libc::c_ulong = 0x5413;

fn get_terminal_size() -> Option<(u16, u16)> {
    let fd = File::open("/dev/tty")
        .or_else(|_| File::open("/dev/stdout"))
        .ok()?;
    let mut ws: Winsize = unsafe { mem::zeroed() };

    let result = unsafe { libc::ioctl(fd.as_raw_fd(), TIOCGWINSZ, &mut ws) };

    if result == 0 {
        Some((ws.ws_row, ws.ws_col))
    } else {
        None
    }
}

static mut RESIZED_PTR: *const AtomicBool = std::ptr::null();

extern "C" fn handle_sigwinch(_: i32) {
    unsafe {
        if !RESIZED_PTR.is_null() {
            (*RESIZED_PTR).store(true, Ordering::SeqCst);
        }
    }
}

extern "C" fn handle_sigint(_: i32) {
    println!("\nReceived Ctrl-C. Exiting.");
    std::process::exit(0);
}

fn main() {
    let resized = Arc::new(AtomicBool::new(true));

    // Register SIGWINCH handler
    unsafe {
        RESIZED_PTR = Arc::as_ptr(&resized);
        libc::signal(SIGWINCH, handle_sigwinch as usize);
    }

    // Register SIGINT (Ctrl-C) handler
    unsafe {
        libc::signal(SIGINT, handle_sigint as usize);
    }

    loop {
        if resized.swap(false, Ordering::SeqCst) {
            if let Some((rows, cols)) = get_terminal_size() {
                println!("Terminal resized: {} rows, {} cols", rows, cols);
            } else {
                println!("Failed to get terminal size");
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}
