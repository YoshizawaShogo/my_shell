pub(crate) static BUILTIN: &[&str] = &["cd", "popd", "abbr", "alias", "history", "setenv", "env"];

use std::env;
use std::path::{Path, PathBuf};

use crate::expansion::Expansion;

pub(crate) fn cd(args: &[String], pipein: &str, cd_history: &mut Vec<PathBuf>) {
    let dir = match (!pipein.is_empty(), args.iter().as_slice()) {
        (true, []) => pipein.to_string(), // pipeinに入力があって、argsが空
        (false, [d]) => d.to_string(),    // pipein空文字かつargsに1要素
        (false, []) => env::var("HOME").unwrap(),
        _ => return, // それ以外はエラー扱い
    };
    let current_dir = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };
    let path = Path::new(&dir);
    match env::set_current_dir(&path) {
        Ok(()) => cd_history.push(current_dir),
        Err(e) => eprintln!("cd: '{}': {}", dir, e),
    }
}

pub(crate) fn popd(cd_history: &mut Vec<PathBuf>) {
    if cd_history.is_empty() {
        eprintln!("popd: cd_stack is empty.");
        return;
    }
    let dir = cd_history.pop().unwrap();
    match env::set_current_dir(&dir) {
        Ok(()) => (),
        Err(e) => eprintln!("popd: '{}': {}", dir.to_str().unwrap().to_string(), e),
    }
}

pub(crate) fn abbr(args: &[String], abbrs: &mut Expansion, is_dirty: &mut bool) {
    match args.len() {
        0 => {
            abbrs.display();
        }
        2 => {
            abbrs.insert(args[0].clone(), args[1].clone(), is_dirty);
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  abbr                  # Show current abbreviations");
            eprintln!("  abbr <name> <value>   # Register abbreviation");
        }
    }
}

pub(crate)  fn alias(args: &[String], aliases: &mut Expansion, is_dirty: &mut bool) {
    match args.len() {
        0 => {
            aliases.display();
        }
        2 => {
            aliases.insert(args[0].clone(), args[1].clone(), is_dirty);
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  alias                  # Show current aliases");
            eprintln!("  alias <name> <value>   # Register alias");
        }
    }
}

pub(crate) fn show_history(history: &[String]) {
    println!("{}", history.join("\n"));
}

pub(crate) fn set_env(args: &[String]) {
    match args.len() {
        2 => {
            unsafe { std::env::set_var(args[0].clone(), args[1].clone()) };
        }
        _ => {
            // 不正な引数の数
            eprintln!("Usage:");
            eprintln!("  setenv <variable> <value>   # set env");
        }
    }
}

pub(crate) fn show_env() {
    for (key, value) in std::env::vars() {
        println!("{}={}", key, value);
    }
}
