use std::{
    env,
    io::Read,
    process::{Command, Stdio},
};

use crate::{
    command::{
        builtin::BUILTIN,
        parse::{CommandExpr, Expr, Redirection},
    },
    shell::MyShell,
};

use std::process::Child;

pub(crate) fn execute(expr: &Expr, shell: &mut MyShell) -> i32 {
    match expr {
        Expr::And(lhs, rhs) => {
            if execute(lhs, shell) == 0 {
                execute(rhs, shell)
            } else {
                1
            }
        }
        Expr::Or(lhs, rhs) => {
            if execute(lhs, shell) != 0 {
                execute(rhs, shell)
            } else {
                0
            }
        }
        Expr::Pipe(commands) => execute_pipeline(commands, shell),
    }
}

fn execute_pipeline(commands: &[CommandExpr], shell: &mut MyShell) -> i32 {
    if commands.is_empty() {
        return 0;
    }
    let mut children: Vec<Child> = Vec::new();
    let mut prev_stdout: Option<std::process::ChildStdout> = None;
    let mut tmp_dirty = false;

    for (i, cmd) in commands.iter().enumerate() {
        let cmd_name = &cmd.cmd_name;

        // 1. BUILTIN の実行
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
            execute_builtin(cmd_name, &cmd.argv, pipein, shell, &mut tmp_dirty);
            continue;
        }

        // 2. stdin の設定 (前のプロセスのstdoutをパイプとして受け取る or 継承)
        let stdin = prev_stdout
            .take()
            .map(Stdio::from)
            .unwrap_or_else(Stdio::inherit);

        // 3. Command の組み立て
        let mut cmd_proc = Command::new(&cmd_name);
        let mut argv = cmd.argv.clone();
        use regex::Regex;
        let re = Regex::new(r"\$([A-Za-z_]\w*)").unwrap();

        for arg in &mut argv {
            if let Some(caps) = re.captures(arg) {
                // キャプチャ位置を取得
                if let Some(m) = caps.get(0) {
                    let start = m.start();
                    // 直前がバックスラッシュならスキップ
                    if start > 0 && arg.as_bytes()[start - 1] == b'\\' {
                        continue;
                    }
                    let var_name = &caps[1];
                    if let Ok(val) = env::var(var_name) {
                        *arg = arg.replace(&format!("${}", var_name), &val);
                    }
                }
            }
        }

        cmd_proc.args(&argv).stdin(stdin);

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
                    eprintln!("Failed to open stderr file '{}': {}", path, e);
                    std::process::exit(1);
                });
                cmd_proc.stderr(file);
            }
        }

        // 6. プロセス起動
        let mut child = match cmd_proc.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to start command '{}': {}", cmd_name, e);
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
                eprintln!("Failed to wait for child process: {}", e);
                last_status = 1;
            }
        }
    }
    last_status
}

fn execute_builtin(cmd: &str, args: &[String], pipein: &str, shell: &mut MyShell, is_dirty: &mut bool) {
    match cmd {
        "cd" => {
            crate::command::builtin::cd(args, pipein, &mut shell.dir_stack);
        }
        "popd" => {
            crate::command::builtin::popd(&mut shell.dir_stack);
        }
        "abbr" => {
            crate::command::builtin::abbr(args, &mut shell.abbrs, is_dirty);
        }
        "alias" => {
            crate::command::builtin::alias(args, &mut shell.aliases, is_dirty);
        }
        "history" => {
            crate::command::builtin::show_history(&shell.history.log.make_contiguous());
        }
        "setenv" => {
            crate::command::builtin::set_env(args);
        }
        "env" => {
            crate::command::builtin::show_env();
        }
        _ => unreachable!(),
    }
}
