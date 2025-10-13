use super::Shell;
use super::{Builtin, BuiltinResult};
use crate::shell::history::History;

pub struct HistoryCmd;

impl Builtin for HistoryCmd {
    fn name(&self) -> &'static str {
        "history"
    }

    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        show_history_with_args(&shell.history, argv)
    }
}

fn show_history_with_args(history: &History, args: &[String]) -> BuiltinResult {
    match args {
        [] => show_history(history),
        _ => BuiltinResult {
            stdout: String::new(),
            stderr: String::from("Usage:\n  history    # Show history\n"),
            code: 1,
        },
    }
}

fn show_history(history: &History) -> BuiltinResult {
    if history.log.is_empty() {
        return BuiltinResult {
            stdout: String::from("(no history)\n"),
            stderr: String::new(),
            code: 0,
        };
    }

    let mut stdout = String::new();
    for (idx, entry) in &history.log {
        stdout.push_str(&format!("{idx}: {entry}\n"));
    }

    BuiltinResult {
        stdout,
        stderr: String::new(),
        code: 0,
    }
}
