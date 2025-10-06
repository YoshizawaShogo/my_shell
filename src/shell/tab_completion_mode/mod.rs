use std::sync::{Arc, Mutex};

use crate::shell::Shell;

pub mod complete;
pub mod exe_list;

pub use complete::DisplayEntry;

pub fn tab_completion_mode(
    shell: &Arc<Mutex<Shell>>,
    buffer: &mut String,
    cursor: &mut usize,
) -> Option<Vec<DisplayEntry>> {
    complete::complete(shell, buffer, cursor)
}
