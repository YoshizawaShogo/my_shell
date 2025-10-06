use std::{collections::BTreeMap, path::PathBuf};

use crate::shell::{
    completion::{CompletionStore, commands_from_expr, default_completion_path},
    expansion::{Abbrs, Aliases},
    history::History,
    tab_completion_mode::exe_list::ExeList,
};

pub mod builtins;
pub mod completion;
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
    pub completion: CompletionStore,
    pub variables: BTreeMap<String, String>,
    pub dir_stack: Vec<PathBuf>,
    pub exit_requested: bool,
}

impl Shell {
    pub fn new() -> Self {
        let completion_path = default_completion_path();
        let mut shell = Self {
            history: History::new(1000),
            abbrs: Abbrs::new("abbr".into()),
            aliases: Aliases::new("aliases".into()),
            exe_list: ExeList::new(),
            completion: CompletionStore::load(completion_path),
            variables: BTreeMap::new(),
            dir_stack: Vec::new(),
            exit_requested: false,
        };
        for name in crate::shell::builtins::name_list() {
            shell.exe_list.insert(name.to_string());
        }
        shell
    }
    pub fn start(&mut self) {}

    pub fn record_completion_from_expr(&mut self, expr: &crate::shell::pipeline::parse::Expr) {
        for tokens in commands_from_expr(expr) {
            self.completion.record_tokens(&tokens);
        }
    }

    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.history.store();
        let _ = self.completion.save();
    }
}
