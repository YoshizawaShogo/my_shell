use std::sync::{Arc, Mutex};
use std::{
    env,
    path::{Path, PathBuf},
};

use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
};

pub struct CdCmd;

impl Builtin for CdCmd {
    fn name(&self) -> &'static str {
        "cd"
    }

    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        let mut sh = shell.lock().unwrap();
        cd(argv, &mut sh, io)
    }
}

fn cd(args: &[String], sh: &mut Shell, io: &mut Io) -> i32 {
    let dir = match args {
        // cd <dir>
        [d] => d.to_string(),
        // cd  （HOMEへ）
        [] => match env::var("HOME") {
            Ok(h) => h,
            Err(e) => {
                let _ = writeln!(io.stderr, "cd: HOME is not set: {}", e);
                return 1;
            }
        },
        _ => {
            let _ = write!(
                io.stderr,
                r#"Usage:
  cd <dir>
  cd        # cd $HOME
"#
            );
            return 1;
        }
    };

    let current_dir: Option<PathBuf> = env::current_dir().ok();
    let path = Path::new(&dir);

    match (env::set_current_dir(path), current_dir) {
        (Ok(()), Some(prev)) => {
            sh.dir_stack.push(prev);
            0
        }
        (Ok(()), None) => 0, // カレント取得は失敗したが cd 自体は成功
        (Err(e), _) => {
            let _ = writeln!(io.stderr, "cd: '{}': {}", dir, e);
            1
        }
    }
}
