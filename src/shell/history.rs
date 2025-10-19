use std::{
    collections::{BTreeSet, VecDeque},
    env,
    fs::{self, File},
    path::Path,
};

use crate::error::Result;

pub struct History {
    log_path: String,
    capacity: usize,
    pub log: VecDeque<(String, String)>, // abs-path, command
    hash: BTreeSet<(String, String)>,
    pub index_up: usize,
    buffer_up: String,
    pub index_r: usize,
    buffer_r: Vec<String>,
}

impl History {
    pub(super) fn load() -> Self {
        let path = env::var("MY_SHELL_HISTORY").unwrap_or_else(|_| {
            env::var("HOME").expect("HOME not set") + "/" + ".my_shell_history"
        });
        let capacity: usize = env::var("MY_SHELL_HISTORY_CAPACITY")
            .ok()
            .and_then(|x| x.parse().ok())
            .unwrap_or(1000);
        if !Path::new(&path).is_file() {
            File::create(&path).unwrap();
        }
        let mut command_log = VecDeque::new();
        let mut hash = BTreeSet::new();
        for line in fs::read_to_string(&path).unwrap().split("\n") {
            if !line.contains(",") {
                continue;
            }
            let mut parts = line.splitn(2, ',');
            let left = parts.next().unwrap().to_string();
            let right = parts.next().unwrap().to_string();
            hash.insert((left.clone(), right.clone()));
            command_log.push_back((left, right));
        }
        Self {
            log_path: path,
            capacity,
            log: command_log,
            hash,
            index_up: 0,
            buffer_up: String::new(),
            index_r: 0,
            buffer_r: vec![],
        }
    }
    pub fn push(&mut self, cmd: String) {
        let pwd = match std::env::current_dir() {
            Ok(p) => p.to_string_lossy().into_owned(),
            Err(_) => return,
        };
        let value = (pwd, cmd);
        if self.hash.contains(&value) {
            let i = self.log.iter().position(|x| x == &value).unwrap();
            self.log.remove(i);
        } else {
            self.hash.insert(value.clone());
        }
        self.log.push_back(value);
        while self.log.len() > self.capacity {
            let poped = self.log.pop_front().unwrap();
            self.hash.remove(&poped);
        }
        self.index_up = 0;
    }
    pub fn prev_up(&mut self, buffer: &str) -> String {
        if self.log.is_empty() {
            return String::new();
        }
        if self.index_up == 0 {
            // 現在打ち込んでいるコマンドラインが消えないように
            self.buffer_up = buffer.to_string();
        }
        if self.index_up < self.log.len() {
            self.index_up += 1;
        }
        self.log[self.log.len() - self.index_up].clone().1
    }
    pub fn next_down(&mut self) -> String {
        if 0 < self.index_up {
            self.index_up -= 1;
        }
        if self.index_up == 0 {
            self.buffer_up.clone()
        } else {
            self.log[self.log.len() - self.index_up].clone().1
        }
    }
    pub fn prev_r(&mut self, buffer: &str) -> String {
        if self.index_r == 0 {
            self.buffer_r = buffer.split_whitespace().map(|x| x.to_string()).collect();
        }
        if self.buffer_r.is_empty() {
            return buffer.to_string();
        }

        self.index_r += 1;
        while self.index_r <= self.log.len() {
            let target = self.log[self.log.len() - self.index_r].clone().1;
            if self.buffer_r.iter().all(|x| target.contains(x)) {
                return target;
            }
            self.index_r += 1;
        }
        "".to_string()
    }
    pub(super) fn save(&self) -> Result<()> {
        let log_str = self
            .log
            .iter()
            .cloned()
            .map(|(pwd, cmd)| pwd + "," + &cmd)
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&self.log_path, log_str)?;
        Ok(())
    }
    pub(super) fn get_ghost(&self, buffer: &str) -> String {
        if buffer.is_empty() {
            return "".into();
        }
        let pwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        let mut fallback = "";

        for (dir, cmd) in self.log.iter().rev() {
            if !cmd.starts_with(buffer) {
                continue;
            }

            // pwdが取れていて、かつ一致したら即リターン
            if let Some(ref p) = pwd
                && dir == p
            {
                fallback = cmd;
                break;
            }
            // 全体の最新候補（最初に見つかったもの）を保存
            if fallback.is_empty() {
                fallback = cmd;
            }
        }
        fallback.strip_prefix(buffer).unwrap_or("").to_string()
    }
}
