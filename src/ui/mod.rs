mod action;
mod term;

use std::io::Write;
use std::io::stdout;

use term::term_size::read_terminal_size;

use crate::ui::term::ansi;
use crate::ui::term::ansi::delete_buffer;
use crate::ui::term::color::Color;
use crate::ui::term::color::fg;
pub(super) use action::Action;
pub(super) use action::Mode;
pub(super) use action::wait_actions;
pub(super) use term::term_mode::{set_origin_term, set_raw_term};

pub fn init() {
    term::init();
}

pub fn flush() {
    stdout().lock().flush().unwrap();
}

pub fn print_newline() {
    write!(stdout().lock(), "{}", term::ansi::newline()).unwrap();
}

pub fn print_prompt() {
    write!(stdout().lock(), "{}", term::prompt::get_prompt()).unwrap();
}

pub fn print_command_line(buffer: &str, cursor: usize, ghost: &str) {
    // format = "{buffer}{gray_color}{ghost}{reset_color}"
    let width = read_terminal_size().width;
    let mut out = stdout().lock();
    write!(out, "{}", buffer).unwrap();
    let ghost_len = ghost.len();
    if !ghost.is_empty() {
        let gray = fg(Color::BrightBlack);
        let reset = fg(Color::Reset);
        write!(out, "{gray}{ghost}{reset}",).unwrap();
    }
    let display_len = buffer.len() + ghost_len;
    if display_len.is_multiple_of(width as usize) && display_len != 0 {
        write!(out, "{}", ansi::scroll_up(1)).unwrap();
    }
    write!(out, "{}", ansi::cursor_back(display_len)).unwrap();
    write!(out, "{}", ansi::cursor_down(cursor as u32 / width as u32)).unwrap();
    write!(out, "{}", ansi::cursor_right(cursor as u32 % width as u32)).unwrap();
}

pub fn clean_term() {
    write!(stdout().lock(), "{}", ansi::clear()).unwrap();
}

pub fn delete_printing(cursor: usize) {
    write!(stdout().lock(), "{}", delete_buffer(cursor)).unwrap();
}
