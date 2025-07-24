pub static BUILTIN: &[&str] = &["cd"];

use std::env;
use std::io::Write;
use std::path::Path;

pub fn cd(dir: &str) {
    let path = Path::new(dir);
    env::set_current_dir(&path).unwrap();
}

// pub fn cd(dir: &str, mut stderr: impl Write) {
//     let path = Path::new(dir);

//     if let Err(e) = env::set_current_dir(&path) {
//         writeln!(stderr, "cd: {}: {}", dir, e).ok();
//     } else {
//         // writeln!(stdout, "Changed directory to {}", dir).ok(); // 任意
//     }
// }
