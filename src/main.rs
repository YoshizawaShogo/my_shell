use std::{
    env,
    io::{stderr, stdin, stdout},
    sync::{Arc, Mutex},
};

use crate::{
    output::{init, term_mode::set_origin_term},
    shell::{
        Shell,
        builtins::{Io, source::source_with_io},
        line_edit_mode::line_edit_mode,
    },
};

pub mod error;
pub mod input;
pub mod output;
pub mod shell;

struct Dropper;

impl Drop for Dropper {
    fn drop(&mut self) {
        set_origin_term();
    }
}

fn main() {
    init();
    let _d = Dropper;
    let shell = Arc::new(Mutex::new(Shell::new()));
    let rc_path = env::var("MY_SHELL_RC")
        .unwrap_or_else(|_| env::var("HOME").expect("HOME not set") + "/" + ".my_shell_rc");
    source_with_io(
        &[rc_path],
        &shell,
        &mut Io {
            stdin: &mut stdin(),
            stdout: &mut stdout(),
            stderr: &mut stderr(),
        },
    );
    line_edit_mode(&shell);
}
