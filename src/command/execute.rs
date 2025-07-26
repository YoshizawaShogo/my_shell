use std::{
    io::Read,
    process::{Command, Stdio},
};

use crate::command::{
    builtin::BUILTIN,
    parse::{CommandExpr, Expr, Redirection},
};

use std::process::Child;

pub fn execute(expr: &Expr) -> i32 {
    match expr {
        Expr::And(lhs, rhs) => {
            if execute(lhs) == 0 {
                execute(rhs)
            } else {
                1
            }
        }
        Expr::Or(lhs, rhs) => {
            if execute(lhs) != 0 {
                execute(rhs)
            } else {
                0
            }
        }
        Expr::Pipe(commands) => execute_pipeline(commands),
    }
}

pub fn execute_pipeline(commands: &[CommandExpr]) -> i32 {
    if commands.is_empty() {
        return 0;
    }
    let mut children: Vec<Child> = Vec::new();
    let mut prev_stdout: Option<std::process::ChildStdout> = None;

    for (i, cmd) in commands.iter().enumerate() {
        // 1. BUILTIN の実行
        let cmd_name = &cmd.argv[0];
        if BUILTIN.contains(&cmd_name.as_str()) {
            if let Some(prev_child) = children.last_mut() {
                let _ = prev_child.wait(); // 結果は無視
            }
            // パイプ入力を文字列化
            let mut pipein = String::new();
            if let Some(mut out) = prev_stdout.take() {
                let _ = out.read_to_string(&mut pipein);
            }
            let pipein = pipein.split('\n').next().unwrap_or("");
            execute_builtin(cmd_name, &cmd.argv[1..], pipein);
            continue;
        }

        // 2. stdin の設定 (前のプロセスのstdoutをパイプとして受け取る or 継承)
        let stdin = prev_stdout
            .take()
            .map(Stdio::from)
            .unwrap_or_else(Stdio::inherit);

        // 3. Command の組み立て
        let mut cmd_proc = Command::new(&cmd.argv[0]);
        cmd_proc.args(&cmd.argv[1..]).stdin(stdin);

        // 4. stdout のリダイレクト／パイプ／継承
        match &cmd.stdout {
            Redirection::Pipe => {
                if i == commands.len() - 1 {
                    cmd_proc.stdout(Stdio::inherit());
                } else {
                    cmd_proc.stdout(Stdio::piped());
                }
            }
            Redirection::Inherit => {
                cmd_proc.stdout(Stdio::inherit());
            }
            Redirection::File { path, append } => {
                let file = if *append {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                } else {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(path)
                };

                let file = file.unwrap_or_else(|e| {
                    eprintln!("stdout ファイル '{}' を開けません: {}", path, e);
                    std::process::exit(1);
                });
                cmd_proc.stdout(file);
            }
        }

        // 5. stderr のリダイレクト／パイプ／継承
        match &cmd.stderr {
            Redirection::Pipe => {
                cmd_proc.stderr(Stdio::piped());
            }
            Redirection::Inherit => {
                cmd_proc.stderr(Stdio::inherit());
            }
            Redirection::File { path, append } => {
                let file = if *append {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                } else {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(path)
                };
                let file = file.unwrap_or_else(|e| {
                    eprintln!("stderr ファイル '{}' を開けません: {}", path, e);
                    std::process::exit(1);
                });
                cmd_proc.stderr(file);
            }
        }

        // 6. プロセス起動
        let mut child = match cmd_proc.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("コマンド '{}' の起動失敗: {}", cmd.argv[0], e);
                return 1;
            }
        };

        // 7. 次のコマンドへのパイプ接続準備
        if i != commands.len() - 1 {
            prev_stdout = child.stdout.take();
        }
        children.push(child);
    }

    // 全 child プロセスを待って終了コードを取得
    let mut last_status = 0;
    for mut child in children {
        match child.wait() {
            Ok(status) => {
                if let Some(code) = status.code() {
                    last_status = code;
                }
            }
            Err(e) => {
                eprintln!("子プロセスの待機に失敗: {}", e);
                last_status = 1;
            }
        }
    }
    last_status
}

fn execute_builtin(cmd: &str, args: &[String], pipein: &str) {
    match cmd {
        "cd" => {
            let dir = match (!pipein.is_empty(), args.iter().as_slice()) {
                (true, []) => pipein, // pipeinに入力があって、argsが空
                (false, [d]) => d,    // pipein空文字かつargsに1要素
                _ => return,          // それ以外はエラー扱い
            };
            crate::command::builtin::cd(dir);
        }
        _ => unreachable!(),
    }
}
