use super::{Builtin, BuiltinResult};
use crate::shell::Shell;

pub struct SetenvCmd;

impl Builtin for SetenvCmd {
    fn name(&self) -> &'static str {
        "setenv"
    }

    fn run(&self, _shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        set_env(argv)
    }
}

fn set_env(args: &[String]) -> BuiltinResult {
    match args {
        [key, value] => {
            unsafe {
                std::env::set_var(key, value);
            }
            BuiltinResult {
                stdout: String::new(),
                stderr: String::new(),
                code: 0,
            }
        }
        _ => BuiltinResult {
            stdout: String::new(),
            stderr: String::from("Usage:\n  setenv <variable> <value>\n"),
            code: 1,
        },
    }
}
