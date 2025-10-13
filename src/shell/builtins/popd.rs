use std::env;

use super::{Builtin, BuiltinResult};
use crate::shell::Shell;

pub struct PopdCmd;

impl Builtin for PopdCmd {
    fn name(&self) -> &'static str {
        "popd"
    }

    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        popd_with_args(shell, argv)
    }
}

fn popd_with_args(shell: &mut Shell, args: &[String]) -> BuiltinResult {
    match args {
        [] => popd(shell),
        _ => BuiltinResult {
            stdout: String::new(),
            stderr: String::from("Usage:\n  popd    # cd \"previous directory\"\n"),
            code: 1,
        },
    }
}

fn popd(shell: &mut Shell) -> BuiltinResult {
    let Some(dir) = shell.dir_stack.last().cloned() else {
        return BuiltinResult {
            stdout: String::new(),
            stderr: String::from("popd: dir_stack is empty.\n"),
            code: 1,
        };
    };

    match env::set_current_dir(&dir) {
        Ok(()) => {
            let _ = shell.dir_stack.pop();
            BuiltinResult {
                stdout: String::new(),
                stderr: String::new(),
                code: 0,
            }
        }
        Err(e) => BuiltinResult {
            stdout: String::new(),
            stderr: format!("popd: '{}': {}\n", dir.display(), e),
            code: 2,
        },
    }
}
