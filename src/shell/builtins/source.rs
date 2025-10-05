use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
};
use std::sync::{Arc, Mutex};

pub struct SourceCmd;

impl Builtin for SourceCmd {
    fn name(&self) -> &'static str {
        "source"
    }

    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        let mut sh = shell.lock().unwrap();
        source(argv, &mut sh, io)
    }
}

fn source(args: &[String], shell: &mut Shell, io: &mut Io) -> i32 {
    let path = match args {
        [path] => path.clone(),
        _ => {
            let _ = write!(
                io.stderr,
                r#"Usage:
  source <path>
"#
            );
            return 1;
        }
    };

    shell.source(&path, io)
}
