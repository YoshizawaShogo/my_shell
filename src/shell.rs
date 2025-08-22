use std::collections::{BTreeSet, VecDeque};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write, stdin, stdout};
use std::path::{Path, PathBuf};

use crate::completion::{ls_starts_with, split_path, Executables};
use crate::expansion::{Abbrs, Aliases, Expansion};
use crate::prompt::display_prompt;
use crate::term_size::read_terminal_size;
use crate::{command, term_mode};

const RC_FILE: &str = ".my_shell_rc";
const HISTORY_FILE: &str = ".my_shell_history";

pub struct MyShell {
    pub(crate) history: History,
    pub(crate) abbrs: Abbrs,
    pub(crate) aliases: Aliases,
    executables: Executables,
    buffer: String,
    cursor: usize,
    pub(crate) dir_stack: Vec<PathBuf>,
}
pub(crate) struct History {
    log_path: String,
    capacity: usize,
    pub(crate) log: VecDeque<String>,
    hash: BTreeSet<String>,
    index: usize,
}

impl History {
    fn new(capacity: usize) -> Self {
        let history_path = env::var("HOME").unwrap() + "/" + HISTORY_FILE;
        if !Path::new(&history_path).is_file() {
            File::create(&history_path).unwrap();
        }
        let mut command_log = VecDeque::new();
        let mut hash = BTreeSet::new();
        for line in fs::read_to_string(&history_path).unwrap().split("\n") {
            hash.insert(line.to_string());
            command_log.push_back(line.to_string());
        }
        Self {
            log_path: history_path,
            capacity,
            log: command_log,
            hash,
            index: 0,
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
        self.index = 0;
    }
    fn prev(&mut self) -> String {
        if self.log.len() - 1 > self.index {
            self.index += 1;
        }
        self.log[self.log.len() - self.index].clone()
    }
    fn next(&mut self) -> String {
        if 1 < self.index {
            self.index -= 1;
        }
        if self.index == 0 {
            self.index = 1;
        }
        self.log[self.log.len() - self.index].clone()
    }
    fn store(&self) {
        let log_str = self.log.iter().cloned().collect::<Vec<_>>().join("\n");
        fs::write(&self.log_path, log_str).unwrap();
    }
}

impl MyShell {
    pub fn new() -> Self {
        Self {
            history: History::new(1000),
            abbrs: Expansion::new("abbreviations".into()),
            aliases: Expansion::new("aliases".into()),
            executables: Executables::new(),
            buffer: String::new(),
            cursor: 0,
            dir_stack: Vec::new(),
        }
    }
    fn expand_abbr(&mut self) {
        if let Some(expanded) = self.abbrs.get(&self.buffer) {
            self.cursor += expanded.len() - self.buffer.len();
            self.buffer = expanded.clone();
        }
    }
    fn execute(&mut self, input: &str) -> i32 {
        let tokens = command::tokenize::Tokens::from(input);
        if tokens.is_empty() {
            return 0;
        }
        let expr = tokens.parse(&self.aliases);
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
        display_prompt(read_terminal_size().width.into());

        while let Some(b) = stdin().lock().by_ref().bytes().next() {
            let b = b.unwrap();
            let mut out = String::new();
            out += &self.back_to_start_point(self.buffer.len(), read_terminal_size().width.into());
            out += &self.delete_after();
            write!(stdout().lock(), "{}", out).unwrap();
            match b {
                // 0   => , // Ctrl + @      (NUL: Null)
                1 => {
                    self.cursor = 0;
                } // Ctrl + A      (SOH: Start of Heading)
                2 => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                } // Ctrl + B      (STX: Start of Text)
                3 => {
                    let mut out = String::new();
                    out += &self
                        .back_to_start_point(self.buffer.len(), read_terminal_size().width.into());
                    out += &self.delete_after();
                    write!(stdout().lock(), "{}", out).unwrap();
                    self.buffer.clear();
                    self.cursor = 0;
                } // Ctrl + C      (ETX: End of Text / Interrupt)
                4 => {
                    if self.buffer.is_empty() {
                        self.history.store();
                        return;
                    } else if self.cursor != self.buffer.len() {
                        self.buffer.remove(self.cursor);
                    }
                } // Ctrl + D      (EOT: End of Transmission / EOF)
                5 => {
                    self.cursor = self.buffer.len();
                } // Ctrl + E      (ENQ: Enquiry)
                6 => {
                    if self.cursor == self.buffer.len() {
                        if self.buffer.is_empty() {
                            continue;
                        }
                        if let Some(h) = self.find_history_rev() {
                            self.buffer = h.clone();
                            self.cursor = self.buffer.len();
                        }
                    } else {
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
                9 => {
                    self.completion_mode();
                } // Ctrl + I      (HT: Horizontal Tab)
                10 => {
                    if !self.buffer.contains(" ") {
                        self.expand_abbr();
                    }
                    self.display_buffer(read_terminal_size().width.into(), false);
                    write!(stdout().lock(), "\r\n").unwrap();
                    self.execute(&self.buffer.clone());
                    self.history.push(self.buffer.clone());
                    self.buffer.clear();
                    self.cursor = 0;
                    display_prompt(read_terminal_size().width.into());
                } // Ctrl + J      (LF: Line Feed / Newline)
                // 11   => , // Ctrl + K      (VT: Vertical Tab)
                // 12   => , // Ctrl + L      (FF: Form Feed / Clear screen)
                13 => {} // Ctrl + M      (CR: Carriage Return)
                14 => {
                    self.buffer = self.history.next();
                    self.cursor = self.buffer.len();
                } // Ctrl + N      (SO: Shift Out)
                // 15   => , // Ctrl + O      (SI: Shift In)
                16 => {
                    self.buffer = self.history.prev();
                    self.cursor = self.buffer.len();
                } // Ctrl + P      (DLE: Data Link Escape)
                // 17   => , // Ctrl + Q      (DC1: XON / Resume transmission)
                // 18   => , // Ctrl + R      (DC2)
                // 19   => , // Ctrl + S      (DC3: XOFF / Pause transmission)
                // 20   => , // Ctrl + T      (DC4)
                // 21   => , // Ctrl + U      (NAK: Negative Acknowledge)
                // 22   => , // Ctrl + V      (SYN: Synchronous Idle)
                23 => {
                    if self.buffer.is_empty() {
                        continue;
                    }
                    let mut buffer = self.buffer.clone();
                    buffer.pop();
                    self.cursor -= 1;
                    while let Some(c) = buffer.chars().last() {
                        if !"/ ".contains(c) {
                            buffer.pop();
                            self.cursor -= 1;
                        } else {
                            break;
                        }
                    }
                    self.buffer = buffer;
                } // Ctrl + W      (ETB: End of Transmission Block)
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
            self.display_buffer(read_terminal_size().width.into(), true);
            stdout().lock().flush().unwrap();
        }
    }

    fn display_buffer(&self, width: usize, completion_flag: bool) {
        let origin = &self.buffer;
        let origin_len = self.buffer.len();
        let output_str = if !self.buffer.is_empty() && completion_flag {
            if let Some(h) = self.find_history_rev() {
                h
            } else {
                origin
            }
        } else {
            origin
        };

        let mut i = 0;
        let mut out = "\x1b[G".to_string();

        let mut output_chars = output_str.chars();
        while let Some(c) = output_chars.next() {
            if i == origin_len {
                out.push_str("\x1b[90m"); // 明るい黒=薄い灰色
            }
            if i == width {
                out.push_str("\r\n");
            }
            out.push(c);
            i += 1;
        }
        if i != 0 && i % width == 0 {
            out.push_str("\r\n");
        }
        out.push_str("\x1b[0m"); // 色リセット

        // 3) カーソル移動
        out += &self.back_to_start_point(output_str.len(), width);
        let mut cursor = self.cursor.clone();
        while cursor >= width {
            out += "\x1b[1B";
            cursor -= width;
        }
        out += &"\x1b[1C".repeat(cursor);

        write!(stdout().lock(), "{}", out).unwrap();
    }

    fn back_to_start_point(&self, buffer_len: usize, width: usize) -> String {
        let row = buffer_len / width;
        "\x1b[1A".repeat(row).to_string() + "\x1b[G"
    }
    fn delete_after(&self) -> String {
        "\x1b[0J".to_string()
    }
    fn find_history_rev(&self) -> Option<&String> {
        self.history
            .log
            .iter()
            .rev()
            .find(|h| h.starts_with(&self.buffer))
    }
    fn completion_mode(&mut self) {
        let target = &self.buffer[..self.cursor];
        let tokens = command::tokenize::Tokens::from(target);
        if tokens.is_empty() {
            return;
        }
        let expr = tokens.parse(&self.aliases);
        let last_cmd_expr = expr.last_cmd_expr();

        let last_char = self.buffer.chars().last().unwrap();
        let args_is_empty = last_cmd_expr.argv.is_empty();
        let last_word = if args_is_empty {
            last_cmd_expr.cmd_name
        } else {
            last_cmd_expr.argv.last().unwrap().clone()
        };

        let (candidates, current) = match (last_char, args_is_empty, last_word.contains("/")) {
            (' ', _, _) => (fs::read_dir(Path::new("./"))
                .unwrap()
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect(), "".to_string()),
            (_, true, false) => (self.executables.completion(&last_word, &self.abbrs, &self.aliases), last_word.clone()),
            (_, true, true) => {
                let (dir, current) = split_path(&last_word);
                let dir = if dir.is_empty() {
                    "./".to_string()
                } else {
                    dir
                };
                (ls_starts_with(&dir, &current, true), current)
            }
            (_, false, _) => {
                let (dir, current) = split_path(&last_word);
                let dir = if dir.is_empty() {
                    "./".to_string()
                } else {
                    dir
                };
                (ls_starts_with(&dir, &current, false), current)
            }
        };

        if candidates.len() == 0 {
            return;
        } else if candidates.len() == 1 {
            let new = candidates.into_iter().next().unwrap();
            let mut last_word = last_word.clone();
            for c in new.chars().skip(current.len()) {
                last_word.push(c);
                self.buffer.push(c);
                self.cursor += 1;
            }
            if Path::new(&last_word).is_dir() {
                self.buffer.push('/');
                self.cursor += 1;
            } else {
                self.buffer.push(' ');
                self.cursor += 1;
            }
            return;
        } else {
            let common_prefix = crate::completion::common_prefix(&candidates);
            if let Some(new) = common_prefix {
                for c in new.chars().skip(current.len()) {
                    self.buffer.push(c);
                    self.cursor += 1;
                }
                crate::completion::print_highlighted_set(&candidates, &new);
            }
        }
    }
}

