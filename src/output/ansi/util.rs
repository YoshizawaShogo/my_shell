use crate::output::term_size::read_terminal_size;

pub fn strip_ansi(s: &str) -> String {
    use regex::Regex;
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

#[inline]
pub fn cursor_up(n: u32) -> String {
    if n > 0 {
        format!("\x1b[{}A", n)
    } else {
        String::new()
    }
}

#[inline]
pub fn cursor_down(n: u32) -> String {
    if n > 0 {
        format!("\x1b[{}B", n)
    } else {
        String::new()
    }
}

#[inline]
pub fn cursor_right(n: u32) -> String {
    if n > 0 {
        format!("\x1b[{}C", n)
    } else {
        String::new()
    }
}

#[inline]
pub fn cursor_left(n: u32) -> String {
    if n > 0 {
        format!("\x1b[{}D", n)
    } else {
        String::new()
    }
}

#[inline]
pub fn scroll_up(n: u32) -> String {
    if n > 0 {
        format!("\x1b[{}S", n)
    } else {
        String::new()
    }
}

#[inline]
pub fn scroll_down(n: u32) -> String {
    if n > 0 {
        format!("\x1b[{}T", n)
    } else {
        String::new()
    }
}

#[inline]
pub fn cursor_to_0_0() -> String {
    "\x1b[H".to_string()
}

#[inline]
pub fn delete_line() -> String {
    "\x1b[2K".to_string()
}

#[inline]
pub fn delete_after() -> String {
    "\x1b[0J".to_string()
}

#[inline]
pub fn newline() -> String {
    "\r\n".to_string()
}

#[inline]
pub fn cursor_to_line_start() -> String {
    "\x1b[G".to_string()
}

#[inline]
pub fn clear() -> String {
    let height: u32 = read_terminal_size().height.into();
    format!("{}{}", scroll_up(height), cursor_to_0_0())
}

#[inline]
pub fn cursor_back(cursor: usize) -> String {
    let width: usize = read_terminal_size().width.into();
    let row = cursor / width;
    cursor_up(row as u32) + &cursor_to_line_start()
}

#[inline]
pub fn delete_buffer(cursor: usize) -> String {
    cursor_back(cursor) + &delete_after()
}
