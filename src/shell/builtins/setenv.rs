use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
};
use std::sync::{Arc, Mutex};

pub struct SetenvCmd;

impl Builtin for SetenvCmd {
    fn name(&self) -> &'static str {
        "setenv"
    }

    fn run(&self, _shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        set_env(argv, io)
    }
}

fn set_env(args: &[String], io: &mut Io) -> i32 {
    match args {
        [key, value] => {
            unsafe {
                std::env::set_var(key, value);
            }
            0
        }
        _ => {
            let _ = write!(
                io.stderr,
                r#"Usage:
  setenv <variable> <value>
"#
            );
            1
        }
    }
}
