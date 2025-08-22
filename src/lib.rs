pub(crate) mod expansion;
pub(crate) mod command;
pub(crate) mod prompt;
pub mod shell;
pub(crate) mod term_mode;
pub(crate) mod term_size;
pub(crate) mod completion;

pub fn init() {
    term_mode::init();
    term_size::init();
}
