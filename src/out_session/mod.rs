pub(crate) mod ansi;
pub(crate) mod term_mode;
pub(crate) mod term_size;
pub(crate) mod color;

use std::io::{self, BufWriter, Write};

pub struct OutSession<'a> {
    buf: BufWriter<io::StdoutLock<'a>>,
}

impl<'a> Write for OutSession<'a> {
    #[inline]
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.buf.write(b)
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.buf.flush()
    }
}

impl<'a> Drop for OutSession<'a> {
    fn drop(&mut self) {
        let _ = self.buf.flush();
    } // セッション終了時に確実に flush
}

