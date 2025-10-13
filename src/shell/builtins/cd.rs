use std::{
    env,
    path::{Path, PathBuf},
};

use super::{Builtin, BuiltinResult};
use crate::shell::Shell;

pub struct CdCmd;

impl Builtin for CdCmd {
    fn name(&self) -> &'static str {
        "cd"
    }

    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        cd(argv, shell)
    }
}

fn cd(args: &[String], sh: &mut Shell) -> BuiltinResult {
    let dir = match args {
        // cd <dir>
        [d] => d.to_string(),
        // cd  （HOMEへ）
        [] => match env::var("HOME") {
            Ok(h) => h,
            Err(e) => {
                return BuiltinResult {
                    stdout: String::new(),
                    stderr: format!("cd: HOME is not set: {}\n", e),
                    code: 1,
                };
            }
        },
        _ => {
            return BuiltinResult {
                stdout: String::new(),
                stderr: String::from("Usage:\n  cd <dir>\n  cd        # cd $HOME\n"),
                code: 1,
            };
        }
    };

    let current_dir: Option<PathBuf> = env::current_dir().ok();
    let path = Path::new(&dir);

    match (env::set_current_dir(path), current_dir) {
        (Ok(()), Some(prev)) => {
            sh.dir_stack.push(prev);
            BuiltinResult {
                stdout: String::new(),
                stderr: String::new(),
                code: 0,
            }
        }
        (Ok(()), None) => BuiltinResult {
            stdout: String::new(),
            stderr: String::new(),
            code: 0,
        }, // カレント取得は失敗したが cd 自体は成功
        (Err(e), _) => BuiltinResult {
            stdout: String::new(),
            stderr: format!("cd: '{}': {}\n", dir, e),
            code: 1,
        },
    }
}
