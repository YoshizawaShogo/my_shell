use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
};
use std::sync::{Arc, Mutex};

pub struct AliasCmd;

impl Builtin for AliasCmd {
    fn name(&self) -> &'static str {
        "alias"
    }

    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        let mut sh = shell.lock().unwrap();
        alias(argv, &mut sh, io)
    }
}

fn alias(args: &[String], shell: &mut Shell, io: &mut Io) -> i32 {
    match args.len() {
        0 => {
            let _ = shell.aliases.display(io.stdout);
            0
        }
        2 => {
            let name = args[0].clone();
            let value = args[1].clone();
            shell.aliases.insert(name, value, &mut shell.exe_list);
            0
        }
        _ => {
            let _ = write!(
                io.stderr,
                r#"Usage:
  alias                  # Show current aliases
  alias <name> <value>   # Register alias
"#
            );
            1
        }
    }
}
