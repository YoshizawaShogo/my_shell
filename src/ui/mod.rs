mod action;
mod term;

use std::io::Write;
use std::io::stdout;

use crate::ui::term::ansi;
use crate::ui::term::ansi::cursor_right;
use crate::ui::term::ansi::cursor_to_line_start;
use crate::ui::term::ansi::cursor_up;
use crate::ui::term::ansi::delete_buffer;
use crate::ui::term::ansi::newline;
use crate::ui::term::color::Color;
use crate::ui::term::color::bg;
use crate::ui::term::color::fg;
pub(super) use action::Action;
pub(super) use action::Mode;
pub(super) use action::wait_actions;
pub(super) use term::term_mode::{set_origin_term, set_raw_term};
pub(super) use term::term_size::read_terminal_size;

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

pub fn delete_after() {
    write!(stdout().lock(), "{}", ansi::delete_after()).unwrap();
}

pub fn print_hat_c() {
    write!(stdout().lock(), "{}^C{}", fg(Color::BrightBlack), fg(Color::Reset)).unwrap();
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

pub fn print_candidates(
    candidates: &Vec<String>,
    cursor: usize,
    index: Option<usize>,
    fixed_len: usize,
) -> usize {
    if candidates.len() <= 1 {
        return 0;
    }
    let size = read_terminal_size();
    let (term_height, term_width) = (size.height as usize, size.width as usize);

    let mut o_width = 1;
    let mut o_max_lens = vec![candidates.iter().map(|line| line.len()).max().unwrap()];
    let mut o_height = candidates.len();
    let mut x_width = term_width / 2;
    while o_width + 1 < x_width {
        let m = (o_width + x_width) / 2;
        let mut max_lens = vec![0; m];
        let chunks = candidates.chunks(m);
        let height = chunks.len();
        for line in chunks {
            for i in 0..line.len() {
                let w = &line[i];
                let l = w.len() + 1;
                max_lens[i] = max_lens[i].max(l);
            }
        }
        let width = max_lens.iter().sum::<usize>() - 1;
        if width <= term_width {
            o_width = m;
            o_max_lens = max_lens;
            o_height = height;
        } else {
            x_width = m;
        }
    }

    if o_height > term_height {
        let buffer = "Too many candidates, can't output";
        print_buffer_and_back(buffer, cursor);
        return 0;
    }

    let mut chunks = candidates.chunks(o_width);
    let mut buffer = "".to_string();
    for i in 0..o_height {
        let line = chunks.next().unwrap();
        for j in 0..line.len() {
            let gray = fg(Color::BrightBlack);
            let reset = fg(Color::Reset);
            let w = &line[j];
            let space = o_max_lens[j] - (j + 1 == line.len()) as usize - w.len();

            let w = if Some(i * o_width + j) == index {
                &format!(
                    "{}{}{w}{reset}",
                    fg(Color::BrightWhite),
                    bg(Color::BrightBlack)
                )
            } else if fixed_len < w.len() {
                &format!(
                    "{gray}{}{reset}{}{reset}{}{reset}",
                    &w[..fixed_len],
                    &w[fixed_len..=fixed_len],
                    &w[fixed_len + 1..]
                )
            } else {
                &format!("{}{w}{reset}", fg(Color::BrightMagenta))
            };
            buffer += &format!("{w}{}", " ".repeat(space));
        }
        if i + 1 != o_height {
            buffer += &newline();
        }
    }

    print_buffer_and_back(&buffer, cursor);
    return o_width;
}

fn print_buffer_and_back(buffer: &str, cursor: usize) {
    let width = read_terminal_size().width as usize;
    let newline = &newline();
    let up = cursor_up(buffer.matches("\r\n").count() as u32 + 1);
    let left_end = cursor_to_line_start();
    let right = cursor_right((cursor % width) as u32);
    write!(stdout().lock(), "{newline}{buffer}{up}{left_end}{right}").unwrap();
}

