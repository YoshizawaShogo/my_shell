use std::io::{self, BufWriter, Write};

use crate::error::Result;
use crate::out_session::{color::Color, OutSession};

#[allow(unused)]
impl<'a> OutSession<'a> {
    #[inline]
    pub fn new_stdout() -> Self {
        OutSession {
            buf: BufWriter::new(io::stdout().lock()),
        }
    }
    #[inline]
    pub fn write_char(&mut self, c: char) -> Result<()> {
        write!(self.buf, "{c}")?;
        Ok(())
    }
    #[inline]
    pub fn cursor_up(&mut self, n: u32) -> Result<()> {
        if n > 0 {
            write!(self.buf, "\x1b[{}A", n)?;
        }
        Ok(())
    }
    #[inline]
    pub fn cursor_down(&mut self, n: u32) -> Result<()> {
        if n > 0 {
            write!(self.buf, "\x1b[{}B", n)?;
        }
        Ok(())
    }
    #[inline]
    pub fn cursor_right(&mut self, n: u32) -> Result<()> {
        if n > 0 {
            write!(self.buf, "\x1b[{}C", n)?;
        }
        Ok(())
    }
    #[inline]
    pub fn cursor_left(&mut self, n: u32) -> Result<()> {
        if n > 0 {
            write!(self.buf, "\x1b[{}D", n)?;
        }
        Ok(())
    }
    #[inline]
    pub fn set_color(&mut self, color: Color) -> Result<()> {
        write!(self.buf, "\x1b[{}m", color.as_code())?;
        Ok(())
    }
    #[inline]
    pub fn reset_color(&mut self) -> Result<()> {
        self.set_color(Color::Reset)?;
        Ok(())
    }
    #[inline]
    pub fn clear_line(&mut self) -> Result<()> {
        write!(self.buf, "\x1b[2K")?;
        Ok(())
    }
    #[inline]
    pub fn clear_after(&mut self) -> Result<()> {
        write!(self.buf, "\x1b[0J")?;
        Ok(())
    }
    #[inline]
    pub fn newline(&mut self) -> Result<()> {
        write!(self.buf, "\r\n")?;
        Ok(())
    }
    #[inline]
    pub fn cursor_to_line_start(&mut self) -> Result<()> {
        write!(self.buf, "\x1b[G")?;
        Ok(())
    }
    #[inline]
    pub fn cursor_to_0_0(&mut self) -> Result<()> {
        write!(self.buf, "\x1b[H")?;
        Ok(())
    }
    #[inline]
    pub fn clear_all(&mut self) -> Result<()> {
        self.cursor_to_0_0()?;
        self.clear_after()?;
        Ok(())
    }
    pub fn back_to_start_point(&mut self, len: usize, width: usize) -> Result<()> {
        let row = len / width;
        self.cursor_up(row as u32)?;
        self.cursor_to_line_start()?;
        Ok(())
    }
}

impl Color {
    fn as_code(&self) -> &'static str {
        match self {
            Color::Reset => "0",
            Color::Black => "30",
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::White => "37",
            Color::BrightBlack => "90",
            Color::BrightRed => "91",
            Color::BrightGreen => "92",
            Color::BrightYellow => "93",
            Color::BrightBlue => "94",
            Color::BrightMagenta => "95",
            Color::BrightCyan => "96",
            Color::BrightWhite => "97",
        }
    }
}