pub static BUILTIN: &[&str] = &["cd", "abbr", "alias"];

use std::collections::HashMap;
use std::env;
use std::path::Path;

pub fn cd(args: &[String], pipein: &str) {
    let dir = match (!pipein.is_empty(), args.iter().as_slice()) {
        (true, []) => pipein, // pipeinに入力があって、argsが空
        (false, [d]) => d,    // pipein空文字かつargsに1要素
        (false, []) => {
            let home = env::var("HOME").unwrap_or_default();
            env::set_current_dir(&Path::new(&home)).unwrap();
            return;
        }
        _ => return,          // それ以外はエラー扱い
    };
    let path = Path::new(dir);
    match env::set_current_dir(&path) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("cd: '{}': {}", e, dir);
        }
    }
}

pub fn register_abbr(args: &[String], abbrs: &mut HashMap<String, String>) {
    match args.len() {
        0 => {
            // 登録済み一覧を表示
            if abbrs.is_empty() {
                println!("No abbreviations registered.");
            } else {
                for (abbr, full) in abbrs {
                    println!("{} => {}", abbr, full);
                }
            }
        }
        2 => {
            // 新しい省略語を登録
            abbrs.insert(args[0].clone(), args[1].clone());
        }
        _ => {
            // 不正な引数数
            eprintln!("Usage:");
            eprintln!("  abbr                  # Show current abbreviations");
            eprintln!("  abbr <name> <value>   # Register abbreviation");
        }
    }
}

pub fn register_alias(args: &[String], aliases: &mut HashMap<String, String>) {
    match args.len() {
        0 => {
            // 登録済み一覧を表示
            if aliases.is_empty() {
                println!("No abbreviations registered.");
            } else {
                for (alias, full) in aliases {
                    println!("{} => {}", alias, full);
                }
            }
        }
        2 => {
            // 新しい省略語を登録
            aliases.insert(args[0].clone(), args[1].clone());
        }
        _ => {
            // 不正な引数数
            eprintln!("Usage:");
            eprintln!("  alias                  # Show current aliases");
            eprintln!("  alias <name> <value>   # Register alias");
        }
    }
}