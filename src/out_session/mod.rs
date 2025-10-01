pub(crate) mod ansi;
pub(crate) mod color;
pub(crate) mod prompt;
pub(crate) mod term_mode;
pub(crate) mod term_size;

use std::io::{self, BufWriter, Write};

use crate::error::Result;
use crate::history::History;
use crate::out_session::color::Color;

pub struct OutSession<'a> {
    buf: BufWriter<io::StdoutLock<'a>>,
}

impl<'a> Drop for OutSession<'a> {
    fn drop(&mut self) {
        let _ = self.buf.flush();
    } // セッション終了時に確実に flush
}

impl<'a> OutSession<'a> {
    #[inline]
    pub(crate) fn flush(&mut self) -> Result<()> {
        self.buf.flush()?;
        Ok(())
    }
    #[inline]
    pub(crate) fn display_buffer(
        &mut self,
        buffer: &str,
        cursor: usize,
        history: &History,
        completion_flag: bool,
    ) -> Result<()> {
        let width = term_size::read_terminal_size().width as usize;
        let origin = buffer.to_string();
        if origin.len() == 0 {
            return Ok(());
        }
        let origin_len = origin.len();
        let output_str = if !origin.is_empty() && completion_flag {
            history.find_history_rev(&origin).unwrap_or(&origin)
        } else {
            &origin
        };

        let mut i = 0;
        let mut chars = output_str.chars();
        while let Some(c) = chars.next() {
            if i == origin_len {
                self.set_color(Color::BrightBlack)?;
            }
            if i == width {
                self.newline()?;
            }
            self.write_char(c)?;
            i += 1;
        }
        self.set_color(Color::Reset)?;

        // 3) カーソル移動
        self.back_to_start_point(output_str.len(), width)?;
        let mut cursor = cursor.clone();
        while cursor >= width {
            self.cursor_up(1)?;
            cursor -= width;
        }
        self.cursor_right(cursor as u32)?;
        Ok(())
    }
}
