use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write, stdin, stdout};
use std::path::Path;

use crate::prompt::get_prompt;
use crate::term_size::read_terminal_size;
use crate::{command, term_mode};

const RC_FILE: &str = ".my_shell_rc";
const LOG_FILE: &str = ".my_shell_log";

pub struct MyShell {
    log: Log,
    abbr: HashMap<String, String>,
}
struct Log {
    log_path: String,
    capacity: usize,
    log: VecDeque<String>,
    hash: HashSet<String>,
}

impl Log {
    fn new(capacity: usize) -> Self {
        let log_path = env::var("HOME").unwrap() + "/" + LOG_FILE;
        if !Path::new(&log_path).is_file() {
            File::create(&log_path).unwrap();
        }
        let mut log = VecDeque::new();
        let mut hash = HashSet::new();
        for line in fs::read_to_string(&log_path).unwrap().split("\n") {
            hash.insert(line.to_string());
            log.push_back(line.to_string());
        }
        Self {
            log_path: log_path,
            capacity,
            log,
            hash,
        }
    }
    fn push(&mut self, value: String) {
        for line in value.split("\n") {
            if self.hash.contains(line) {
                let i = self.log.iter().position(|x| x == &line).unwrap();
                self.log.remove(i);
            } else {
                self.hash.insert(line.to_string());
            }
            self.log.push_back(line.to_string());
        }
        while self.log.len() > self.capacity {
            let poped = self.log.pop_front().unwrap();
            self.hash.remove(&poped);
        }
    }
    fn store(&self) {
        let log_str = self.log.iter().cloned().collect::<Vec<_>>().join("\n");
        fs::write(&self.log_path, log_str).unwrap();
    }
    // fn ref_logs(&self) -> &VecDeque<String> {
    //     &self.log
    // }
}

impl MyShell {
    pub fn new() -> Self {
        Self {
            log: Log::new(1000),
            abbr: HashMap::new(),
        }
    }
    fn expand_abbr(&self, buffer: &mut String) {
        if let Some(expanded) = self.abbr.get(buffer) {
            *buffer = expanded.clone()
        }
    }
    fn execute(input: &str) -> i32 {
        let tokens = command::tokenize::tokenize(input);
        if tokens.is_empty() {
            return 0;
        }
        let (expr, _) = command::parse::parse(&tokens);
        term_mode::set_origin_term();
        let r = command::execute::execute(&expr);
        term_mode::set_raw_term();
        r
    }
    pub fn command_mode(&mut self) {
        let rc_path = env::var("HOME").unwrap() + "/" + RC_FILE;
        if !Path::new(&rc_path).is_file() {
            File::create(&rc_path).unwrap();
        }
        for line in fs::read_to_string(&rc_path).unwrap().split("\n") {
            Self::execute(line);
        }
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
                        self.log.store();
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
                    Self::execute(&buffer);
                    self.log.push(buffer.clone());
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
                32..=126 => {
                    if b == 32 {
                        if !buffer.contains(" ") {
                            self.expand_abbr(&mut buffer);
                        }
                    }
                    buffer.insert(cursor, b as char);
                    cursor += 1;
                }
                // 127   => , // Ctrl + ?      (DEL: Delete)
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
