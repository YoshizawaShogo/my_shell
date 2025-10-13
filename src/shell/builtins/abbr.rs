use super::{Builtin, BuiltinResult};
use crate::shell::Shell;

pub struct AbbrCmd;

impl Builtin for AbbrCmd {
    fn name(&self) -> &'static str {
        "abbr"
    }

    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        abbr(argv, shell)
    }
}

fn abbr(args: &[String], sh: &mut Shell) -> BuiltinResult {
    match args.len() {
        0 => {
            let mut buf = Vec::new();
            sh.abbrs.display(&mut buf);
            BuiltinResult {
                stdout: String::from_utf8_lossy(&buf).into_owned(),
                stderr: String::new(),
                code: 0,
            }
        }
        2 => {
            let name = &args[0];
            let value = &args[1];
            sh.abbrs
                .insert(name.clone(), value.clone(), &mut sh.exe_list);
            BuiltinResult {
                stdout: String::new(),
                stderr: String::new(),
                code: 0,
            }
        }
        _ => BuiltinResult {
            stdout: String::new(),
            stderr: String::from(
                "Usage:\n  abbr                  # Show current abbreviations\n  abbr <name> <value>   # Register abbreviation\n",
            ),
            code: 1,
        },
    }
}
