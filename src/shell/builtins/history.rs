use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
    history::History,
};
use std::sync::{Arc, Mutex};

pub struct HistoryCmd;

impl Builtin for HistoryCmd {
    fn name(&self) -> &'static str {
        "history"
    }

    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        let sh = shell.lock().unwrap();
        show_history_with_args(&sh.history, argv, io)
    }
}

fn show_history_with_args(history: &History, args: &[String], io: &mut Io) -> i32 {
    match args {
        [] => show_history(history, io),
        _ => {
            let _ = write!(
                io.stderr,
                r#"Usage:
  history    # Show history
"#
            );
            1
        }
    }
}

fn show_history(history: &History, io: &mut Io) -> i32 {
    if history.log.is_empty() {
        let _ = writeln!(io.stdout, "(no history)");
        return 0;
    }

    for (idx, entry) in &history.log {
        let _ = writeln!(io.stdout, "{idx}: {entry}");
    }

    0
}
