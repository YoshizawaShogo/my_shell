pub static BUILTIN: &[&str] = &["cd"];

use std::env;
use std::path::Path;

pub fn cd(dir: &str) {
    let path = Path::new(dir);
    match env::set_current_dir(&path) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("cd: '{}': {}", e, dir);
        }
    }
}
