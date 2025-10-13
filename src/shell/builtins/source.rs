use super::{Builtin, BuiltinResult};
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{execute, parse, shell::Shell, tokenize};

pub struct SourceCmd;

impl Builtin for SourceCmd {
    fn name(&self) -> &'static str {
        "source"
    }

    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        source_with_io(argv, shell)
    }
}

pub fn source_with_io(args: &[String], shell: &mut Shell) -> BuiltinResult {
    let path = match args {
        [path] => path,
        _ => {
            return BuiltinResult {
                stdout: String::new(),
                stderr: String::from("Usage:\n  source <path>\n"),
                code: 1,
            };
        }
    };

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return BuiltinResult {
                stdout: String::new(),
                stderr: format!("source: cannot open '{}': {}\n", path, e),
                code: 1,
            };
        }
    };

    let reader = BufReader::new(file);
    let mut last_status = 0;

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(s) => s,
            Err(e) => {
                return BuiltinResult {
                    stdout: String::new(),
                    stderr: format!("source: read error: {}\n", e),
                    code: 1,
                };
            }
        };

        // 空行・（任意）コメント行はスキップ
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // 1) tokenize
        let tokens = tokenize(trimmed);

        // 2) parse
        let Ok(expr) = parse(&tokens) else {
            return BuiltinResult {
                stdout: String::new(),
                stderr: format!("source: parse error in line: {}\n", trimmed),
                code: 1,
            };
        };

        // 5) execute
        match execute(&expr, shell) {
            Ok(code) => last_status = code,
            Err(e) => {
                return BuiltinResult {
                    stdout: String::new(),
                    stderr: format!("source: execute error: {:?}\n", e),
                    code: 1,
                };
            }
        }
    }

    BuiltinResult {
        stdout: String::new(),
        stderr: String::new(),
        code: last_status,
    }
}
