use crate::shell::Shell;

mod abbr;
mod alias;
mod cd;
mod exit;
mod history;
mod popd;
mod set;
mod setenv;
mod source;
mod complete;

pub use source::source_with_io;

pub struct BuiltinResult {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

pub trait Builtin {
    fn name(&self) -> &'static str;
    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult;
}

// すべての builtin を登録
fn registry() -> &'static [&'static dyn Builtin] {
    &[
        &abbr::AbbrCmd,
        &alias::AliasCmd,
        &cd::CdCmd,
        &exit::ExitCmd,
        &history::HistoryCmd,
        &popd::PopdCmd,
        &set::SetCmd,
        &setenv::SetenvCmd,
        &source::SourceCmd,
        &complete::CompleteCmd,
    ]
}

pub fn find(name: &str) -> Option<&'static dyn Builtin> {
    registry().iter().copied().find(|b| b.name() == name)
}

pub fn name_list() -> Vec<&'static str> {
    registry().iter().map(|x| x.name()).collect()
}
