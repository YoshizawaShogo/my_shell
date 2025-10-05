use std::sync::{Arc, Mutex};

use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
};

pub struct AbbrCmd;

impl Builtin for AbbrCmd {
    fn name(&self) -> &'static str {
        "abbr"
    }

    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        let mut sh = shell.lock().unwrap();
        abbr(argv, &mut sh, io)
    }
}

fn abbr(args: &[String], sh: &mut Shell, io: &mut Io) -> i32 {
    match args.len() {
        0 => {
            let _ = sh.abbrs.display(io.stdout);
            0
        }
        2 => {
            let name = &args[0];
            let value = &args[1];
            sh.abbrs
                .insert(name.clone(), value.clone(), &mut sh.exe_list);
            0
        }
        _ => {
            let _ = writeln!(
                io.stderr,
                r#"Usage:
  abbr                  # Show current abbreviations
  abbr <name> <value>   # Register abbreviation"#
            );
            1
        }
    }
}
