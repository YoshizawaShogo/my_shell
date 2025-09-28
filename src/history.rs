use std::{
    collections::{BTreeSet, VecDeque},
    env,
    fs::{self, File},
    path::Path,
};

const HISTORY_FILE: &str = ".my_shell_history";

pub(crate) struct History {
    log_path: String,
    capacity: usize,
    pub(crate) log: VecDeque<(String, String)>, // abs-path, command
    hash: BTreeSet<(String, String)>,
    index: usize,
}

impl History {
    pub(crate) fn new(capacity: usize) -> Self {
        let history_path = env::var("HOME").unwrap() + "/" + HISTORY_FILE;
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
        }
    }
    pub(crate) fn push(&mut self, pwd: String, cmd: String) {
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
    pub(crate) fn prev(&mut self) -> String {
        if self.log.len() - 1 > self.index {
            self.index += 1;
        }
        self.log[self.log.len() - self.index].clone().1
    }
    pub(crate) fn next(&mut self) -> String {
        if 0 < self.index {
            self.index -= 1;
        }
        if self.index == 0 {
            self.index = 1;
        }
        self.log[self.log.len() - self.index].clone().1
    }
    pub(crate) fn store(&self) {
        let log_str = self
            .log
            .iter()
            .cloned()
            .map(|(pwd, cmd)| pwd + "," + &cmd)
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&self.log_path, log_str).unwrap();
    }
    pub(crate) fn find_history_rev(&self, current: &str) -> Option<&String> {
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
