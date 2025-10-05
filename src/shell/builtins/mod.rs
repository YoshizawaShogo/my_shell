use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};

use crate::shell::Shell;

pub mod abbr;
pub mod alias;
pub mod cd;
pub mod exit;
pub mod history;
pub mod popd;
pub mod set;
pub mod setenv;
pub mod source;

pub trait Builtin: Sync {
    fn name(&self) -> &'static str;
    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32;
}

pub struct Io<'a> {
    pub stdin: &'a mut dyn Read,
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
}

// すべての builtin を登録
fn registry() -> &'static [&'static dyn Builtin] {
    &[
        &crate::shell::builtins::abbr::AbbrCmd,
        &crate::shell::builtins::alias::AliasCmd,
        &crate::shell::builtins::cd::CdCmd,
        &crate::shell::builtins::exit::ExitCmd,
        &crate::shell::builtins::history::HistoryCmd,
        &crate::shell::builtins::popd::PopdCmd,
        &crate::shell::builtins::set::SetCmd,
        &crate::shell::builtins::setenv::SetenvCmd,
        &crate::shell::builtins::source::SourceCmd,
    ]
}

pub fn find(name: &str) -> Option<&'static dyn Builtin> {
    registry().iter().copied().find(|b| b.name() == name)
}

pub fn name_list() -> Vec<&'static str> {
    registry().iter().map(|x| x.name()).collect()
}
