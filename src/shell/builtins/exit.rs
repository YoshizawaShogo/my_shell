use std::sync::{Arc, Mutex};

use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
};

pub struct ExitCmd;

impl Builtin for ExitCmd {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        let mut sh = shell.lock().unwrap();
        exit_with_args(&mut sh, argv, io)
    }
}

fn exit_with_args(shell: &mut Shell, args: &[String], io: &mut Io) -> i32 {
    match args {
        [] => {
            shell.request_exit();
            0
        }
        _ => {
            let _ = writeln!(io.stderr, "Usage:\n  exit    # no args");
            1
        }
    }
}
