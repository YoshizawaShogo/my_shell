use std::collections::BTreeMap;

use super::{Builtin, BuiltinResult};
use crate::shell::Shell;

pub struct SetCmd;

impl Builtin for SetCmd {
    fn name(&self) -> &'static str {
        "set"
    }

    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        set(argv, &mut shell.variables)
    }
}

fn set(args: &[String], variables: &mut BTreeMap<String, String>) -> BuiltinResult {
    match args {
        [key, value] => {
            variables.insert(key.clone(), value.clone());
            BuiltinResult {
                stdout: String::new(),
                stderr: String::new(),
                code: 0,
            }
        }
        _ => BuiltinResult {
            stdout: String::new(),
            stderr: String::from("Usage:\n  set <variable> <value>\n"),
            code: 1,
        },
    }
}
