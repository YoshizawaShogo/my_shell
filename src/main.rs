use std::sync::{Arc, Mutex};

use crate::{
    output::{init, term_mode::set_origin_term},
    shell::{Shell, line_edit_mode::line_edit_mode},
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
    line_edit_mode(&shell);
}
