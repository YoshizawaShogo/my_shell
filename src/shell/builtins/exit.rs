use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
};
use std::sync::{Arc, Mutex};

pub struct ExitCmd;

impl Builtin for ExitCmd {
    fn name(&self) -> &'static str {
        "exit"
    }
    fn run(&self, _shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        exit_with_args(argv, io)
    }
}

fn exit_with_args(args: &[String], io: &mut Io) -> i32 {
    match args {
        [] => exit(),
        _ => {
            let _ = write!(
                io.stderr,
                r#"Usage:
  exit    # no args
"#
            );
            1
        }
    }
}

fn exit() -> ! {
    std::process::exit(0)
}
