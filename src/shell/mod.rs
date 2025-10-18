pub mod builtins;
pub mod completion;
mod exe_list;
mod expansion;
pub mod history;

use std::{collections::BTreeMap, env, path::PathBuf};

use crate::shell::expansion::{Abbrs, Aliases};

use completion::CompletionStore;
use exe_list::ExeList;
use history::History;

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
        init_env();
        let mut s = Self {
            history: History::load(),
            abbrs: Abbrs::new("abbr".into()),
            aliases: Aliases::new("aliases".into()),
            exe_list: ExeList::new(),
            completion: CompletionStore::load().unwrap(),
            variables: BTreeMap::new(),
            dir_stack: Vec::new(),
            exit_requested: false,
        };
        let rc_path = get_rc_path();
        s.source(rc_path);
        s
    }
    fn source(&mut self, path: String) -> i32 {
        let ret = crate::shell::builtins::source_with_io(&[path], self);
        print!("{}", ret.stdout);
        ret.code
    }
    fn request_exit(&mut self) {
        self.exit_requested = true;
    }
    pub fn get_ghost(&self, buffer: &str) -> String {
        self.history.get_ghost(buffer)
    }
}

fn get_rc_path() -> String {
    env::var("MY_SHELL_RC")
        .unwrap_or_else(|_| env::var("HOME").expect("HOME not set") + "/" + ".my_shell_rc")
}

fn init_env() {
    let shlvl = env::var("SHLVL")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
        + 1;
    unsafe {
        env::set_var("SHLVL", shlvl.to_string());
    }
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
        libc::signal(libc::SIGTSTP, libc::SIG_IGN);
        libc::signal(libc::SIGTTIN, libc::SIG_IGN);
        libc::signal(libc::SIGTTOU, libc::SIG_IGN);
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        let _ = self.history.save();
        let _ = self.completion.save();
    }
}
