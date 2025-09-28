use libc::{SIGWINCH, TIOCGWINSZ, signal};
use std::fs::File;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex, OnceLock};

pub(crate) fn init() {
    init_terminal_size();
    init_sigwinch();
}

#[repr(C)]
#[derive(Debug, Clone)]
pub(crate) struct TerminalSize {
    height: u16,
    pub(crate) width: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            height: 24,
            width: 80,
        }
    }
}
static TERMINAL_SIZE: OnceLock<Arc<Mutex<TerminalSize>>> = OnceLock::new();

fn get_terminal_size() -> Option<TerminalSize> {
    let fd = File::open("/dev/tty")
        .or_else(|_| File::open("/dev/stdout"))
        .ok()?;
    let mut ts: TerminalSize = unsafe { mem::zeroed() };

    let result = unsafe { libc::ioctl(fd.as_raw_fd(), TIOCGWINSZ, &mut ts) };

    if result == 0 {
        Some(TerminalSize {
            height: ts.height,
            width: ts.width,
        })
    } else {
        None
    }
}

extern "C" fn handle_sigwinch(_: i32) {
    if let Some(arc_mutex) = TERMINAL_SIZE.get() {
        let new = get_terminal_size().unwrap_or(TerminalSize::default());
        // println!("{:?}", new);
        *arc_mutex.lock().unwrap() = new;
    }
}

pub(crate) fn read_terminal_size() -> TerminalSize {
    if let Some(arc_mutex) = TERMINAL_SIZE.get() {
        arc_mutex.lock().unwrap().clone()
    } else {
        TerminalSize::default()
    }
}

fn init_terminal_size() {
    let size = Arc::new(Mutex::new(
        get_terminal_size().unwrap_or(TerminalSize::default()),
    ));
    TERMINAL_SIZE.get_or_init(|| size);
}

fn init_sigwinch() {
    unsafe {
        signal(SIGWINCH, handle_sigwinch as usize);
    }
}
