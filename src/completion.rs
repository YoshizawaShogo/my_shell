use std::{collections::BTreeSet, fs, os::unix::fs::PermissionsExt, path::Path};

use crate::{command::builtin::BUILTIN, expansion::{Abbrs, Aliases}};

///  PATH上の実行可能ファイルが変更されることは稀だと考えるので、
/// 考慮しないこととする。
pub(crate) struct Executables {
    executables: BTreeSet<String>,
    pub(crate) is_dirty: bool,
}

impl Executables {
    pub(crate) fn new() -> Self {
        Self {
            executables: BTreeSet::new(),
            is_dirty: true,
        }
    }
    pub(crate) fn update(&mut self, abbrs: &Abbrs, aliases: &Aliases) {
        // abbr, alias, builtin, path上のcmd
        let mut set: BTreeSet<String> = BTreeSet::new();
        set.extend(abbrs.keys().cloned());
        set.extend(aliases.keys().cloned());
        for &cmd in BUILTIN.iter() {
            set.insert(cmd.to_string());
        }
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            set.insert(name.to_string());
                        }
                    }
                }
            }
        }
        self.executables = set;
    }
    pub(crate) fn completion(&mut self, name: &str, abbrs: &Abbrs, aliases: &Aliases) -> BTreeSet<String> {
        if self.is_dirty {
            self.update(abbrs, aliases);
            self.is_dirty = false;
        }
        self.executables.iter().cloned().filter(|x| x.starts_with(&name)).collect()
    }
}

pub(crate) fn ls_starts_with(dir: &str, prefix: &str, only_exec: bool) -> BTreeSet<String> {
    let entries = ls(dir, only_exec);
    if prefix.is_empty() {
        entries
    } else {
        entries
            .into_iter()
            .filter(|name| name.starts_with(prefix))
            .collect()
    }
}

fn is_executable(path: &Path) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        let perm = metadata.permissions();
        // 所有者/グループ/その他の実行権限をチェック
        perm.mode() & 0o111 != 0
    } else {
        false
    }
}

fn ls(dir: &str, only_exec: bool) -> BTreeSet<String> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return BTreeSet::new(),
    };

    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| !only_exec || is_executable(&entry.path()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

pub(crate) fn print_highlighted_set<T: AsRef<str>>(set: &BTreeSet<T>, prefix: &str) {
    let mut out = "[".to_string();
    for item in set.iter() {
        if item.as_ref().starts_with(prefix) {
            let (matched, rest) = item.as_ref().split_at(prefix.len());
            out.push_str(&format!("\x1b[37m{}\x1b[90m{}\x1b[0m  ", matched, rest));
        } else {
            out.push_str(&format!("\x1b[90m{}\x1b[0m  ", item.as_ref()));
        }
    }
    out.push_str("]");
    println!("{}", out);
}

pub(crate) fn common_prefix<T: AsRef<str>>(set: &BTreeSet<T>) -> Option<String> {
    let first = set.iter().next()?.as_ref();
    let mut prefix_len = 0;
    for (i, b) in first.as_bytes().iter().enumerate() {
        if set.iter().all(|s| s.as_ref().as_bytes().get(i) == Some(b)) {
            prefix_len += 1;
        } else {
            break;
        }
    }
    Some(first[..prefix_len].to_string())
}

pub(crate) fn split_path(s: &str) -> (String, String) {
    if s.ends_with('/') {
        (s.trim_end_matches('/').to_string(), "".to_string())
    } else if let Some((dir, file)) = s.rsplit_once('/') {
        (dir.to_string(), file.to_string())
    } else {
        ("".to_string(), s.to_string())
    }
}