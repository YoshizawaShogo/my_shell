#![allow(unused)]
pub enum Color {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Ansi256(u8),
    Rgb(u8, u8, u8),
}

#[inline]
pub fn fg(color: Color) -> String {
    match color {
        Color::Reset => "\x1b[0m".to_string(),
        Color::Black => "\x1b[30m".to_string(),
        Color::Red => "\x1b[31m".to_string(),
        Color::Green => "\x1b[32m".to_string(),
        Color::Yellow => "\x1b[33m".to_string(),
        Color::Blue => "\x1b[34m".to_string(),
        Color::Magenta => "\x1b[35m".to_string(),
        Color::Cyan => "\x1b[36m".to_string(),
        Color::White => "\x1b[37m".to_string(),
        Color::BrightBlack => "\x1b[90m".to_string(),
        Color::BrightRed => "\x1b[91m".to_string(),
        Color::BrightGreen => "\x1b[92m".to_string(),
        Color::BrightYellow => "\x1b[93m".to_string(),
        Color::BrightBlue => "\x1b[94m".to_string(),
        Color::BrightMagenta => "\x1b[95m".to_string(),
        Color::BrightCyan => "\x1b[96m".to_string(),
        Color::BrightWhite => "\x1b[97m".to_string(),
        Color::Ansi256(n) => format!("\x1b[38;5;{n}m"),
        Color::Rgb(r, g, b) => format!("\x1b[38;2;{r};{g};{b}m"),
    }
}

#[inline]
pub fn bg(color: Color) -> String {
    match color {
        Color::Reset => "\x1b[0m".to_string(),
        Color::Black => "\x1b[40m".to_string(),
        Color::Red => "\x1b[41m".to_string(),
        Color::Green => "\x1b[42m".to_string(),
        Color::Yellow => "\x1b[43m".to_string(),
        Color::Blue => "\x1b[44m".to_string(),
        Color::Magenta => "\x1b[45m".to_string(),
        Color::Cyan => "\x1b[46m".to_string(),
        Color::White => "\x1b[47m".to_string(),
        Color::BrightBlack => "\x1b[100m".to_string(),
        Color::BrightRed => "\x1b[101m".to_string(),
        Color::BrightGreen => "\x1b[102m".to_string(),
        Color::BrightYellow => "\x1b[103m".to_string(),
        Color::BrightBlue => "\x1b[104m".to_string(),
        Color::BrightMagenta => "\x1b[105m".to_string(),
        Color::BrightCyan => "\x1b[106m".to_string(),
        Color::BrightWhite => "\x1b[107m".to_string(),
        Color::Ansi256(n) => format!("\x1b[48;5;{n}m"),
        Color::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
    }
}
