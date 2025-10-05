use std::{collections::BTreeMap, path::PathBuf};

use crate::shell::{
    builtins::Io,
    expansion::{Abbrs, Aliases},
    history::History,
    tab_completion_mode::exe_list::ExeList,
};

pub mod builtins;
pub mod expansion;
pub mod history;
pub mod line_edit_mode;
pub mod pipeline;
pub mod tab_completion_mode;

pub struct Shell {
    pub history: History,
    pub abbrs: Abbrs,
    pub aliases: Aliases,
    pub exe_list: ExeList,
    pub variables: BTreeMap<String, String>,
    pub dir_stack: Vec<PathBuf>,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            history: History::new(1000),
            abbrs: Abbrs::new("abbr".into()),
            aliases: Aliases::new("aliases".into()),
            exe_list: ExeList::new(),
            variables: BTreeMap::new(),
            dir_stack: Vec::new(),
        }
    }
    pub fn start(&mut self) {}
    pub fn source(&mut self, path: &str, io: &mut Io) -> i32 {
        0
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.history.store();
    }
}
