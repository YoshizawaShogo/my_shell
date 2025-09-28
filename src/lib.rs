pub(crate) mod command;
pub(crate) mod completion;
pub(crate) mod error;
pub(crate) mod expansion;
pub(crate) mod history;
pub(crate) mod key;
pub(crate) mod out_session;
pub(crate) mod prompt;
pub mod shell;

pub fn init() {
    out_session::term_mode::init();
    out_session::term_size::init();
}
