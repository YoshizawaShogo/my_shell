use super::{Builtin, BuiltinResult};
use crate::shell::Shell;

pub struct ExitCmd;

impl Builtin for ExitCmd {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        exit_with_args(shell, argv)
    }
}

fn exit_with_args(shell: &mut Shell, args: &[String]) -> BuiltinResult {
    match args {
        [] => {
            shell.request_exit();
            BuiltinResult {
                stdout: String::new(),
                stderr: String::new(),
                code: 0,
            }
        }
        _ => BuiltinResult {
            stdout: String::new(),
            stderr: String::from("Usage:\n  exit    # no args\n"),
            code: 1,
        },
    }
}
