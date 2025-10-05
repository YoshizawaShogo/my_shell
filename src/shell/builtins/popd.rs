use std::sync::{Arc, Mutex};
use std::{env, path::PathBuf};

use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
};

pub struct PopdCmd;

impl Builtin for PopdCmd {
    fn name(&self) -> &'static str {
        "popd"
    }

    // 他の builtin と同様に Arc<Mutex<Shell>> を受け取りに統一
    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        popd_with_args(shell, argv, io)
    }
}

fn popd_with_args(shell: &Arc<Mutex<Shell>>, args: &[String], io: &mut Io) -> i32 {
    match args {
        [] => popd(shell, io),
        _ => {
            let _ = write!(
                io.stderr,
                r#"Usage:
  popd    # cd "previous directory"
"#
            );
            1
        }
    }
}

fn popd(shell: &Arc<Mutex<Shell>>, io: &mut Io) -> i32 {
    // 1) まずトップ要素を“見る”だけ（失敗時に壊さないため）
    let target: Option<PathBuf> = {
        let sh = shell.lock().unwrap();
        sh.dir_stack.last().cloned()
    };

    let Some(dir) = target else {
        let _ = writeln!(io.stderr, "popd: dir_stack is empty.");
        return 1;
    };

    // 2) chdir をロック外で実施（ロックは短く）
    match env::set_current_dir(&dir) {
        Ok(()) => {
            // 3) 成功したらポップ（ここで初めてスタックを更新）
            let mut sh = shell.lock().unwrap();
            let _ = sh.dir_stack.pop();
            0
        }
        Err(e) => {
            let _ = writeln!(io.stderr, "popd: '{}': {}", dir.display(), e);
            2
        }
    }
}
