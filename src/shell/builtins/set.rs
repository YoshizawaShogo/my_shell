use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
};

pub struct SetCmd;

impl Builtin for SetCmd {
    fn name(&self) -> &'static str {
        "set"
    }

    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        let mut sh = shell.lock().unwrap();
        set(argv, &mut sh.variables, io)
    }
}

fn set(args: &[String], variables: &mut BTreeMap<String, String>, io: &mut Io) -> i32 {
    match args {
        [key, value] => {
            variables.insert(key.clone(), value.clone());
            0
        }
        _ => {
            let _ = write!(
                io.stderr,
                r#"Usage:
  set <variable> <value>
"#
            );
            1
        }
    }
}
