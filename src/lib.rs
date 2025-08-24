pub(crate) mod command;
pub(crate) mod completion;
pub(crate) mod expansion;
pub(crate) mod history;
pub(crate) mod prompt;
pub mod shell;
pub(crate) mod term_mode;
pub(crate) mod term_size;

pub fn init() {
    term_mode::init();
    term_size::init();
}
