use super::{Builtin, BuiltinResult};
use crate::shell::Shell;

pub struct AliasCmd;

impl Builtin for AliasCmd {
    fn name(&self) -> &'static str {
        "alias"
    }

    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        alias(argv, shell)
    }
}

fn alias(args: &[String], shell: &mut Shell) -> BuiltinResult {
    match args.len() {
        0 => {
            let mut buf = Vec::new();
            shell.aliases.display(&mut buf);
            BuiltinResult {
                stdout: String::from_utf8_lossy(&buf).into_owned(),
                stderr: String::new(),
                code: 0,
            }
        }
        2 => {
            let name = args[0].clone();
            let value = args[1].clone();
            shell.aliases.insert(name, value, &mut shell.exe_list);
            BuiltinResult {
                stdout: String::new(),
                stderr: String::new(),
                code: 0,
            }
        }
        _ => BuiltinResult {
            stdout: String::new(),
            stderr: String::from(
                "Usage:\n  alias                  # Show current aliases\n  alias <name> <value>   # Register alias\n",
            ),
            code: 1,
        },
    }
}
