use libc::{
    tcgetattr, tcsetattr, termios, BRKINT, CS8, CSIZE, ECHO, ECHONL, ICANON, ICRNL, IEXTEN, IGNBRK, IGNCR, INLCR, ISIG, ISTRIP, IXON, OPOST, PARENB, PARMRK, STDIN_FILENO, TCSAFLUSH
};
use std::sync::OnceLock;

static ORIGIN_TERM: OnceLock<termios> = OnceLock::new();
static RAW_TERM: OnceLock<termios> = OnceLock::new();

pub fn init() {
    init_origin_term();
    init_raw_term();
}

fn get_term_mode() -> termios {
    unsafe {
        let mut term: termios = std::mem::zeroed();
        if tcgetattr(STDIN_FILENO, &mut term) != 0 {
            panic!("tcgetattr failed");
        }
        term
    }
}

fn set_term_mode(term: &termios) {
    unsafe {
        if tcsetattr(STDIN_FILENO, TCSAFLUSH, term) != 0 {
            panic!("tcsetattr failed");
        }
    }
}

pub fn set_origin_term() {
    let term = ORIGIN_TERM
        .get()
        .expect("ORIGIN_TERM not initialized");
    set_term_mode(&term);
}

pub fn set_raw_term() {
    // rawモードに切り替え
    let term = RAW_TERM
        .get()
        .expect("ORIGIN_TERM not initialized");
    set_term_mode(&term);
}

fn init_origin_term() {
    let term = get_term_mode();
    ORIGIN_TERM.get_or_init(|| term);
}

fn init_raw_term() {
    // man cfmakeraw に色々書いてある
    let mut base_term = ORIGIN_TERM
        .get()
        .expect("ORIGIN_TERM not initialized")
        .clone();
    base_term.c_iflag &= !(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
    base_term.c_oflag &= !OPOST;
    base_term.c_lflag &= !(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
    base_term.c_cflag &= !(CSIZE | PARENB);
    base_term.c_cflag |= CS8;
    RAW_TERM.get_or_init(|| base_term);
}
