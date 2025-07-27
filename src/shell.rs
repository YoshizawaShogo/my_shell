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
    pub log: Log,
    pub abbrs: HashMap<String, String>,
    pub aliases: HashMap<String, String>,
    pub buffer: String,
    pub cursor: usize,
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
            abbrs: HashMap::new(),
            aliases: HashMap::new(),
            buffer: String::new(),
            cursor: 0,
        }
    }
    fn expand_abbr(&mut self) {
        if let Some(expanded) = self.abbrs.get(&self.buffer) {
            self.cursor += expanded.len() - self.buffer.len();
            self.buffer = expanded.clone();
        }
    }
    fn execute(&mut self, input: &str) -> i32 {
        let tokens = command::tokenize::tokenize(input);
        if tokens.is_empty() {
            return 0;
        }
        let (expr, _) = command::parse::parse(&tokens, &self.aliases);
        term_mode::set_origin_term();
        let r = command::execute::execute(&expr, self);
        term_mode::set_raw_term();
        r
    }
    pub fn command_mode(&mut self) {
        let rc_path = env::var("HOME").unwrap() + "/" + RC_FILE;
        if !Path::new(&rc_path).is_file() {
            File::create(&rc_path).unwrap();
        }
        for line in fs::read_to_string(&rc_path).unwrap().split("\n") {
            self.execute(line);
        }
        term_mode::set_raw_term();
        println!("{}\r", get_prompt(read_terminal_size().width.into()));

        while let Some(b) = stdin().lock().by_ref().bytes().next() {
            let b = b.unwrap();
            self.clear_lines(read_terminal_size().width.into());
            match b {
                // 0   => , // Ctrl + @      (NUL: Null)
                1 => {
                    if self.cursor < self.buffer.len() {
                        self.cursor += 1;
                    }            
                } // Ctrl + A      (SOH: Start of Heading)
                2 => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                } // Ctrl + B      (STX: Start of Text)
                3 => {
                    self.buffer.clear();
                    self.cursor = 0;
                } // Ctrl + C      (ETX: End of Text / Interrupt)
                4 => {
                    if self.buffer.is_empty() {
                        self.log.store();
                        return;
                    } else if self.cursor != self.buffer.len() {
                        self.buffer.remove(self.cursor);
                    }
                } // Ctrl + D      (EOT: End of Transmission / EOF)
                // 5   => , // Ctrl + E      (ENQ: Enquiry)
                6 => {
                    if self.cursor != self.buffer.len() {
                        self.cursor += 1;
                    }
                } // Ctrl + F      (ACK: Acknowledge)
                // 7   => , // Ctrl + G      (BEL: Bell / Beep)
                8 => {
                    if !self.buffer.is_empty() && self.cursor != 0 {
                        self.buffer.remove(self.cursor - 1);
                        self.cursor -= 1;
                    }
                } // Ctrl + H      (BS: Backspace)
                // 9   => , // Ctrl + I      (HT: Horizontal Tab)
                10 => {
                    if !self.buffer.contains(" ") {
                        self.expand_abbr();
                    }
                    self.display_buffer(read_terminal_size().width.into());
                    print!("\r\n");
                    self.execute(&self.buffer.clone());
                    self.log.push(self.buffer.clone());
                    self.buffer.clear();
                    self.cursor = 0;
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
                        if !self.buffer.contains(" ") {
                            self.expand_abbr();
                        }
                    }
                    self.buffer.insert(self.cursor, b as char);
                    self.cursor += 1;
                }
                // 127   => , // Ctrl + ?      (DEL: Delete)
                _ => {}
            }
            self.display_buffer(read_terminal_size().width.into());
            stdout().lock().flush().unwrap();
        }
    }
    fn clear_lines(&mut self, width: usize) {
        let buffer_len = self.buffer.len();
        let mut buf = String::new();

        let num_lines = ((buffer_len + width - 1) / width).saturating_sub(1);

        // カーソルをバッファの先頭行に戻す
        while self.cursor >= width {
            self.cursor -= width;
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

    fn display_buffer(&mut self, width: usize) {
        let mut buffer = self.buffer.clone();
        let num_lines = ((buffer.len() + width - 1) / width).saturating_sub(1);
        let mut buf = String::new();
        while buffer.len() >= width as usize {
            buf += &buffer[..width];
            buf += "\r\n";
            buffer = buffer[width..].to_string();
        }
        buf += &buffer;

        // 行末にいるので、行数分だけ上へ戻す
        for _ in 0..num_lines {
            buf += "\x1b[1A";
        }
        buf += "\x1b[G"; // 行頭へ

        while self.cursor >= width {
            buf += "\x1b[1B";
            self.cursor -= width;
        }

        buf += &"\x1b[1C".repeat(self.cursor);

        write!(stdout().lock(), "{}", buf).unwrap();
    }
}
