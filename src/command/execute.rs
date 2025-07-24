use std::{
    env, io::{BufRead, BufReader, Read}, process::{Command, Stdio}
};

use crate::command::{
    builtin::{BUILTIN, cd},
    parse::{CommandExpr, Expr},
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

fn execute_pipeline(commands: &[CommandExpr]) -> i32 {
    if commands.is_empty() {
        return 0;
    }
    let mut children: Vec<Child> = Vec::new();
    let mut prev_stdout: Option<std::process::ChildStdout> = None;

    for (i, cmd) in commands.iter().enumerate() {
        // 組み込みコマンド
        let cmd_name = &cmd.argv[0];
        if BUILTIN.contains(&cmd_name.as_str()) {
            // 前のプロセスを待つ
            if let Some(prev_child) = children.last_mut() {
                prev_child.wait().ok(); // ignore exit code
            }

            let mut pipein = String::new();
            match prev_stdout.take() {
                Some(mut out) => {
                    out.read_to_string(&mut pipein).unwrap();
                    ()
                }
                None => (),
            };
            let pipein = pipein.split("\n").next().unwrap();
            println!("{} | {}, {:?}", pipein, cmd_name, &cmd.argv[1..]);
            continue;
        }
        
        // 外部コマンド
        let stdin = match prev_stdout.take() {
            Some(out) => Stdio::from(out),
            None => Stdio::inherit(),
        };
        let stdout = if i == commands.len() - 1 {
            Stdio::inherit()
        } else {
            Stdio::piped()
        };
        let mut command = Command::new(&cmd.argv[0]);
        command.args(&cmd.argv[1..]).stdin(stdin).stdout(stdout);

        if let Some(ref path) = cmd.stderr {
            if let Ok(f) = std::fs::File::create(path.0.clone()) {
                command.stderr(f);
            }
        } else {
            command.stderr(Stdio::piped());
        }

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(_) => return 1,
        };

        if i != commands.len() - 1 {
            prev_stdout = child.stdout.take();
        }
        children.push(child);
    }
    let mut last_status: Option<i32> = None;
    for child in children.iter_mut() {
        match child.wait() {
            Ok(status) => {
                last_status = status.code(); // Some(i32) or None if terminated by signal
            }
            Err(e) => {
                eprintln!("Failed to wait on child: {}", e);
                last_status = Some(1); // エラー時は 1 にしておく
            }
        }
    }
    last_status.unwrap_or(1)
}

fn execute_builtin(cmd: &str, args: &str, stdin: &str) {
    match cmd {
        "cd" => {}
        _ => unreachable!(),
    }
}
