pub mod ansi;
pub mod prompt;
pub mod term_mode;
pub mod term_size;

pub fn init() {
    term_mode::init();
    term_size::init();
}
