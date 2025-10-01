use std::path::{Path, PathBuf};
use std::{env, fs};

use libc::exit;

use crate::command;
use crate::completion::{Executables, ls_starts_with, split_path};
use crate::error::Result;
use crate::expansion::{Abbrs, Aliases, Expansion};
use crate::history::History;
use crate::key::{Key, Modifier, wait_keys};
use crate::out_session::OutSession;
use crate::out_session::prompt::display_prompt;
use crate::out_session::term_mode;
use crate::out_session::term_size::read_terminal_size;

pub struct MyShell {
    pub(crate) history: History,
    pub(crate) abbrs: Abbrs,
    pub(crate) aliases: Aliases,
    executables: Executables,
    buffer: String,
    clear_buffer_flag: bool,
    cursor: usize,
    pub(crate) dir_stack: Vec<PathBuf>,
}

impl MyShell {
    pub fn new() -> Self {
        Self {
            history: History::new(1000),
            abbrs: Expansion::new("abbreviations".into()),
            aliases: Expansion::new("aliases".into()),
            executables: Executables::new(),
            buffer: String::new(),
            clear_buffer_flag: true,
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
    pub(crate) fn execute(&mut self, input: &str) -> i32 {
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
    pub(crate) fn source(&mut self, path: &str) {
        for line in fs::read_to_string(&path).unwrap().split("\n") {
            self.execute(line);
        }
    }
    pub fn command_mode(&mut self) -> Result<()> {
        let rc_path = env::var("MY_SHELL_RC")
            .unwrap_or_else(|_| env::var("HOME").expect("HOME not set") + "/" + ".my_shell_rc");
        if Path::new(&rc_path).is_file() {
            self.source(&rc_path);
        }
        term_mode::set_raw_term();
        display_prompt()?;
        OutSession::lock().clear_after()?;
        loop {
            let mut out = OutSession::lock();
            if self.clear_buffer_flag {
                out.back_to_start_point(self.buffer.len(), read_terminal_size().width.into())?;
                out.clear_after()?;
            }
            out.display_buffer(&self.buffer, self.cursor, &self.history, true)?;
            out.flush()?;
            let keys = wait_keys(10).unwrap();
            for key in keys {
                self.parse_key(key)?;
            }
        }
    }

    fn parse_key(&mut self, key: Key) -> Result<()> {
        match key {
            // Ctrl + d
            Key::Char('d', Modifier { ctrl: true, .. }) => {
                if self.buffer.is_empty() {
                    self.history.store();
                    unsafe {
                        exit(0);
                    }
                } else if self.cursor != self.buffer.len() {
                    self.buffer.remove(self.cursor);
                }
            }
            // Ctrl + A : 行頭へ
            Key::Char('a', Modifier { ctrl: true, .. }) | Key::Home(_) => {
                self.cursor = 0;
            }
            // Ctrl + B : ←
            Key::Char('b', Modifier { ctrl: true, .. }) | Key::ArrowLeft(_) => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            // Ctrl + C : 入力行クリア
            Key::Char('c', Modifier { ctrl: true, .. }) => {
                self.buffer.clear();
                self.cursor = 0;
            }
            // Ctrl + E : 行末へ
            Key::Char('e', Modifier { ctrl: true, .. }) | Key::End(_) => {
                self.cursor = self.buffer.len();
            }
            // Ctrl + F : →（ただし行末なら履歴リバース検索）
            Key::Char('f', Modifier { ctrl: true, .. }) | Key::ArrowRight(_) => {
                if self.cursor == self.buffer.len() {
                    if self.buffer.is_empty() {
                        // 何もしない
                    } else if let Some(h) = self.history.find_history_rev(&self.buffer) {
                        self.buffer = h.clone();
                        self.cursor = self.buffer.len();
                    }
                } else {
                    self.cursor += 1;
                }
            }
            // Backspace / Ctrl + H
            Key::Backspace(_)
            | Key::Char('\x08', Modifier { .. })
            | Key::Char('h', Modifier { ctrl: true, .. }) => {
                if !self.buffer.is_empty() && self.cursor != 0 {
                    self.buffer.remove(self.cursor - 1);
                    self.cursor -= 1;
                }
            }
            // Tab / Ctrl + I
            Key::Tab(_) | Key::Char('i', Modifier { ctrl: true, .. }) => {
                self.completion_mode();
            }
            // Enter / Ctrl + J
            Key::Enter(_) | Key::Char('j', Modifier { ctrl: true, .. }) => {
                let mut out = OutSession::lock();
                if !self.buffer.contains(' ') {
                    let old = self.buffer.to_string();
                    self.expand_abbr();
                    let new = self.buffer.to_string();
                    if old != new {
                        out.back_to_start_point(
                            self.buffer.len(),
                            read_terminal_size().width.into(),
                        )?;
                        out.clear_after()?;
                        out.display_buffer(&self.buffer, self.cursor, &self.history, false)?;
                    }
                }
                let pwd = env::current_dir().unwrap().to_string_lossy().into_owned();
                out.newline()?;
                out.flush()?;
                self.execute(&self.buffer.clone());
                self.history.push(pwd, self.buffer.clone());
                self.buffer.clear();
                self.cursor = 0;
                out.display_prompt()?;
            }
            Key::Char('l', Modifier { ctrl: true, .. }) => {
                let mut out = OutSession::lock();
                out.clear_all()?;
                out.display_prompt()?;
                out.display_buffer(&self.buffer, self.cursor, &self.history, true)?;
            }
            // Ctrl + N : 次の履歴
            Key::Char('n', Modifier { ctrl: true, .. }) | Key::ArrowDown(_) => {
                self.buffer = self.history.next();
                self.cursor = self.buffer.len();
            }
            // Ctrl + P : 前の履歴
            Key::Char('p', Modifier { ctrl: true, .. }) | Key::ArrowUp(_) => {
                self.buffer = self.history.prev();
                self.cursor = self.buffer.len();
            }
            // Ctrl + W : 単語削除（直前の / or スペースまで）
            Key::Char('w', Modifier { ctrl: true, .. }) => {
                if self.buffer.is_empty() || self.cursor == 0 {
                    // 何もしない
                } else {
                    // カーソル直前の1文字を落としてから、区切りまで戻す
                    let mut buf = self.buffer.clone();
                    // まず1文字削除（カーソル直前）
                    buf.remove(self.cursor - 1);
                    self.cursor -= 1;

                    while self.cursor > 0 {
                        let c = buf.chars().nth(self.cursor - 1).unwrap();
                        if !"/ ".contains(c) {
                            // さらに1文字削除して左へ
                            // chars().nth() は O(n) なので、本来は byte/idx で持つとよい
                            buf.remove(self.cursor - 1);
                            self.cursor -= 1;
                        } else {
                            break;
                        }
                    }
                    self.buffer = buf;
                }
            }
            // スペース（初回スペース前に略語展開）
            Key::Char(
                ' ',
                Modifier {
                    ctrl: false,
                    alt: false,
                    shift: false,
                },
            ) => {
                if !self.buffer.contains(' ') {
                    self.expand_abbr();
                }
                self.buffer.insert(self.cursor, ' ');
                self.cursor += 1;
            }
            // 可視 ASCII の通常文字挿入
            Key::Char(ch, Modifier { ctrl: false, .. }) if ch.is_ascii_graphic() => {
                self.buffer.insert(self.cursor, ch);
                self.cursor += 1;
            }
            // それ以外は無視
            _ => {}
        }
        Ok(())
    }

    fn completion_mode(&mut self) {
        // 現在のカーソルの位置を判別(cmd, subcmd, -付きオプション, ファイル名, その他(補完対象外))

        // cmd, subcmdを取得 or None
        // cmd, subcmdを元に補完候補が設定されているかどうかを判別
        let target = &self.buffer[..self.cursor];
        let tokens = command::tokenize::Tokens::from(target);
        if tokens.is_empty() {
            return;
        }
        let expr = tokens.parse(&self.aliases);
        let last_cmd_expr = expr.last_cmd_expr();

        let last_char = self.buffer.chars().last().unwrap();
        let args_is_empty = last_cmd_expr.args.is_empty();
        let last_word = if args_is_empty {
            last_cmd_expr.cmd_name
        } else {
            last_cmd_expr.args.last().unwrap().clone()
        };

        let (candidates, current) = match (last_char, args_is_empty, last_word.contains("/")) {
            (' ', _, _) => (
                fs::read_dir(Path::new("./"))
                    .unwrap()
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect(),
                "".to_string(),
            ),
            (_, true, false) => (
                self.executables
                    .completion(&last_word, &self.abbrs, &self.aliases),
                last_word.clone(),
            ),
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
                self.buffer.insert(self.cursor, c);
                self.cursor += 1;
            }
            if Path::new(&last_word).is_dir() {
                self.buffer.insert(self.cursor, '/');
                self.cursor += 1;
            } else {
                if self.buffer.chars().nth(self.cursor) != Some(' ') {
                    self.buffer.insert(self.cursor, ' ');
                }
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
