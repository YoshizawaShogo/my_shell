use std::{
    collections::{BTreeSet, VecDeque},
    env,
    fs::{self, File},
    path::Path,
};

pub struct History {
    log_path: String,
    capacity: usize,
    pub log: VecDeque<(String, String)>, // abs-path, command
    hash: BTreeSet<(String, String)>,
    index: usize,
    buffer: String,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        let history_path = env::var("MY_SHELL_HISTORY").unwrap_or_else(|_| {
            env::var("HOME").expect("HOME not set") + "/" + ".my_shell_history"
        });
        if !Path::new(&history_path).is_file() {
            File::create(&history_path).unwrap();
        }
        let mut command_log = VecDeque::new();
        let mut hash = BTreeSet::new();
        for line in fs::read_to_string(&history_path).unwrap().split("\n") {
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
            log_path: history_path,
            capacity,
            log: command_log,
            hash,
            index: 0,
            buffer: String::new(),
        }
    }
    pub fn push(&mut self, pwd: String, cmd: String) {
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
        self.index = 0;
    }
    pub fn prev(&mut self, buffer: &str) -> String {
        if self.index == 0 {
            // 現在打ち込んでいるコマンドラインが消えないように
            self.buffer = buffer.to_string();
        }
        if self.index + 1 < self.log.len() {
            self.index += 1;
        }
        self.log[self.log.len() - self.index].clone().1
    }
    pub fn next(&mut self) -> String {
        if 0 < self.index {
            self.index -= 1;
        }
        if self.index == 0 {
            self.buffer.clone()
        } else {
            self.log[self.log.len() - self.index].clone().1
        }
    }
    pub fn store(&self) {
        let log_str = self
            .log
            .iter()
            .cloned()
            .map(|(pwd, cmd)| pwd + "," + &cmd)
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&self.log_path, log_str).unwrap();
    }
    pub fn find_history_rev(&self, current: &str) -> Option<&String> {
        let pwd = env::current_dir().unwrap().to_string_lossy().into_owned();
        let first_candidate = self
            .log
            .iter()
            .filter(|x| x.0 == pwd)
            .map(|x| &x.1)
            .rev()
            .find(|h| h.starts_with(current));

        if first_candidate.is_some() {
            return first_candidate;
        }

        self.log
            .iter()
            .map(|x| &x.1)
            .rev()
            .find(|h| h.starts_with(current))
    }
}
