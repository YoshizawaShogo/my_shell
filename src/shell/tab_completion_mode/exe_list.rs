use std::{collections::BTreeSet, env, fs::read_dir, os::unix::fs::PermissionsExt, path::Path};

pub struct ExeList {
    path_entries: BTreeSet<String>,
    extra_entries: BTreeSet<String>,
    pre_path: String,
}

impl ExeList {
    pub fn new() -> Self {
        Self {
            path_entries: BTreeSet::new(),
            extra_entries: BTreeSet::new(),
            pre_path: String::new(),
        }
    }

    pub fn insert(&mut self, executable: String) {
        self.extra_entries.insert(executable);
    }

    pub fn command_candidates(&mut self, prefix: &str) -> Vec<String> {
        self.refresh_path_entries();
        let mut combined = BTreeSet::new();
        combined.extend(self.path_entries.iter().cloned());
        combined.extend(self.extra_entries.iter().cloned());
        combined
            .into_iter()
            .filter(|name| name.starts_with(prefix))
            .collect()
    }

    pub fn all_commands(&mut self) -> Vec<String> {
        self.refresh_path_entries();
        let mut combined = BTreeSet::new();
        combined.extend(self.path_entries.iter().cloned());
        combined.extend(self.extra_entries.iter().cloned());
        combined.into_iter().collect()
    }

    fn refresh_path_entries(&mut self) {
        let path_env = env::var("PATH").unwrap_or_default();
        if path_env == self.pre_path {
            return;
        }
        self.pre_path = path_env.clone();
        self.path_entries.clear();
        for dir in path_env.split(':') {
            if dir.is_empty() {
                continue;
            }
            let path = Path::new(dir);
            let entries = match read_dir(path) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let name = match entry.file_name().into_string() {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() || metadata.is_symlink() {
                        if metadata.permissions().mode() & 0o111 != 0 {
                            self.path_entries.insert(name);
                        }
                    }
                }
            }
        }
    }
}
