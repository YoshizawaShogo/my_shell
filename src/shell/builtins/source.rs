use std::{
    fs::File,
    io::{BufRead, BufReader},
    sync::{Arc, Mutex},
};

use crate::shell::{
    Shell,
    builtins::{Builtin, Io},
    pipeline::{
        execute::execute, parse::parse, pre_execute::expand_expr_with_shell, tokenize::tokenize,
    },
};

pub struct SourceCmd;

impl Builtin for SourceCmd {
    fn name(&self) -> &'static str {
        "source"
    }

    /// ここで lock せず、Arc<Mutex<Shell>> をそのまま渡す
    fn run(&self, shell: &Arc<Mutex<Shell>>, argv: &[String], io: &mut Io) -> i32 {
        source_with_io(argv, shell, io)
    }
}

/// Usage:
///   source <path>
pub fn source_with_io(args: &[String], shell: &Arc<Mutex<Shell>>, io: &mut Io) -> i32 {
    let path = match args {
        [path] => path,
        _ => {
            let _ = write!(io.stderr, "Usage:\n  source <path>\n");
            return 1;
        }
    };

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(io.stderr, "source: cannot open '{}': {}", path, e);
            return 1;
        }
    };

    let reader = BufReader::new(file);
    let mut last_status = 0;

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(s) => s,
            Err(e) => {
                let _ = writeln!(io.stderr, "source: read error: {}", e);
                return 1;
            }
        };

        // 空行・（任意）コメント行はスキップ
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // コメントを無視したい場合は有効化:
        // if trimmed.starts_with('#') { continue; }

        // 1) tokenize
        let tokens = tokenize(trimmed);

        // 3) parse
        let Some((expr, _consumed)) = parse(&tokens) else {
            // パース不能行はエラー扱いにするなら return 1;
            // ここではスキップせずエラーを表示して中断する方針にします
            let _ = writeln!(io.stderr, "source: parse error in line: {}", trimmed);
            return 1;
        };

        // 4) pre_execute（変数展開）
        let expanded = expand_expr_with_shell(&expr, shell);

        // 5) execute
        match execute(&expanded, shell) {
            Ok(code) => last_status = code,
            Err(e) => {
                let _ = writeln!(io.stderr, "source: execute error: {:?}", e);
                return 1;
            }
        }
    }

    last_status
}
