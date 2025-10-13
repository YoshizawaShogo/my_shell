use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
};

use crate::error::Result;

#[derive(Default, Clone, Debug)]
pub struct CompletionStore {
    data: BTreeMap<String, CommandEntry>,
    path: String,
}

#[derive(Default, Clone, Debug)]
pub struct CommandEntry {
    pub options: BTreeSet<String>,
    pub subcommands: BTreeMap<String, SubcommandEntry>,
    pub filter: CompletionFilter,
}

#[derive(Default, Clone, Debug)]
pub struct SubcommandEntry {
    pub options: BTreeSet<String>,
    pub filter: CompletionFilter,
}

#[derive(Clone, Debug, Default)]
pub struct CompletionFilter {
    pub type_filter: Option<char>,
    pub require_exec: bool,
}

impl CompletionStore {
    pub(super) fn load() -> Self {
        let path = env::var("MY_SHELL_COMPLETION").unwrap_or_else(|_| {
            env::var("HOME").expect("HOME not set") + "/" + ".my_shell_completion"
        });
        let mut store = Self {
            data: BTreeMap::new(),
            path,
        };
        store.read_file();
        store
    }
    pub(super) fn save(&self) -> Result<()> {
        Ok(())
    }
    fn read_file(&mut self) {
        let Ok(content) = fs::read_to_string(&self.path) else {
            return;
        };
    }
}
