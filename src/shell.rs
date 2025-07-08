use std::io::{Read, Write, stdin, stdout};

use crate::prompt::get_prompt;
use crate::{command, term_mode};
use crate::term_size::{read_terminal_size};

pub struct MyShell {
}

impl MyShell {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn command_mode(&mut self) {
        term_mode::set_raw_term();
        println!("{}\r", get_prompt(read_terminal_size().width.into()));
        let mut buffer = String::new();
        let mut cursor = 0; // bufferのindex (0..=len)
        while let Some(b) = stdin().lock().by_ref().bytes().next() {
            let b = b.unwrap();
            self.clear_lines(&buffer, cursor, read_terminal_size().width.into());
            match b {
                // 0   => , // Ctrl + @      (NUL: Null)
                1 => {} // Ctrl + A      (SOH: Start of Heading)
                2 => {
                    if cursor != 0 {
                        cursor -= 1;
                    }
                } // Ctrl + B      (STX: Start of Text)
                3 => {
                    buffer.clear();
                    cursor = 0;
                } // Ctrl + C      (ETX: End of Text / Interrupt)
                4 => {
                    if buffer.is_empty() {
                        return;
                    } else if cursor != buffer.len() {
                        buffer.remove(cursor);
                    }
                } // Ctrl + D      (EOT: End of Transmission / EOF)
                // 5   => , // Ctrl + E      (ENQ: Enquiry)
                6 => {
                    if cursor != buffer.len() {
                        cursor += 1;
                    }
                } // Ctrl + F      (ACK: Acknowledge)
                // 7   => , // Ctrl + G      (BEL: Bell / Beep)
                8 => {
                    if !buffer.is_empty() && cursor != 0 {
                        buffer.remove(cursor - 1);
                        cursor -= 1;
                    }
                } // Ctrl + H      (BS: Backspace)
                // 9   => , // Ctrl + I      (HT: Horizontal Tab)
                10 => {
                    self.display_buffer(&buffer, cursor, read_terminal_size().width.into());
                    print!("\r\n");
                    command::execute::run(&buffer);
                    buffer.clear();
                    cursor = 0;
                    println!("\r{}\r", get_prompt(read_terminal_size().width.into()));
                } // Ctrl + J      (LF: Line Feed / Newline)
                // 11   => , // Ctrl + K      (VT: Vertical Tab)
                // 12   => , // Ctrl + L      (FF: Form Feed / Clear screen)
                13 => {} // Ctrl + M      (CR: Carriage Return)
                // 14   => , // Ctrl + N      (SO: Shift Out)
                // 15   => , // Ctrl + O      (SI: Shift In)
                // 16   => , // Ctrl + P      (DLE: Data Link Escape)
                // 17   => , // Ctrl + Q      (DC1: XON / Resume transmission)
                // 18   => , // Ctrl + R      (DC2)
                // 19   => , // Ctrl + S      (DC3: XOFF / Pause transmission)
                // 20   => , // Ctrl + T      (DC4)
                // 21   => , // Ctrl + U      (NAK: Negative Acknowledge)
                // 22   => , // Ctrl + V      (SYN: Synchronous Idle)
                // 23   => , // Ctrl + W      (ETB: End of Transmission Block)
                // 24   => , // Ctrl + X      (CAN: Cancel)
                // 25   => , // Ctrl + Y      (EM: End of Medium)
                // 26   => , // Ctrl + Z      (SUB: Substitute / EOF on Windows)
                // 27   => , // Ctrl + [      (ESC: Escape)
                // 28   => , // Ctrl + \      (FS: File Separator)
                // 29   => , // Ctrl + ]      (GS: Group Separator)
                // 30   => , // Ctrl + ^      (RS: Record Separator)
                // 31   => , // Ctrl + _      (US: Unit Separator)
                // 127   => , // Ctrl + ?      (DEL: Delete)
                32..=126 => {
                    buffer.insert(cursor, b as char);
                    cursor += 1;
                }
                _ => {}
            }
            self.display_buffer(&buffer, cursor, read_terminal_size().width.into());
            stdout().lock().flush().unwrap();
        }
    }
    fn clear_lines(&mut self, buffer: &str, mut cursor: usize, width: usize) {
        let buffer_len = buffer.len();
        let mut buf = String::new();

        let num_lines = ((buffer_len + width - 1) / width).saturating_sub(1);

        // カーソルをバッファの先頭行に戻す
        while cursor >= width {
            cursor -= width;
            buf += "\x1b[1A"; // 上に移動
        }

        buf += "\x1b[2K"; // 現在の行を削除

        // 現在の行を含め、下の行もすべて削除
        for _ in 0..num_lines {
            buf += "\x1b[1B"; // 次の行へ
            buf += "\x1b[2K"; // 現在の行を削除
        }

        // 行末にいるので、行数分だけ上へ戻す
        for _ in 0..num_lines {
            buf += "\x1b[1A";
        }

        // 行頭に戻る（\x1b[G でも良いが、\x1b[H の方が明確）
        buf += "\x1b[G";

        write!(stdout().lock(), "{}", buf).unwrap();
    }

    fn display_buffer(&mut self, mut buffer: &str, mut cursor: usize, width: usize) {
        let num_lines = ((buffer.len() + width - 1) / width).saturating_sub(1);
        let mut buf = String::new();
        while buffer.len() >= width as usize {
            buf += &buffer[..width];
            buf += "\r\n";
            buffer = &buffer[width..];
        }
        buf += &buffer;

        // 行末にいるので、行数分だけ上へ戻す
        for _ in 0..num_lines {
            buf += "\x1b[1A";
        }
        buf += "\x1b[G"; // 行頭へ

        while cursor >= width {
            buf += "\x1b[1B";
            cursor -= width;
        }

        buf += &"\x1b[1C".repeat(cursor);

        write!(stdout().lock(), "{}", buf).unwrap();
    }
}


