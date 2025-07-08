use std::process::{Command, Stdio};

use crate::{command::{parse::{parse, CommandExpr, Expr}, tokenize::tokenize, util::validate_command}, term_mode::{set_origin_term, set_raw_term}};

pub fn run(input: &str) -> i32 {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return 0;
    }
    if let Some(msg) = validate_command(&tokens) {
        eprintln!("{}", msg);
        return 127;
    }
    let (expr, _) = parse(&tokens);
    set_origin_term();
    let r = execute(&expr);
    set_raw_term();
    r
}

fn execute(expr: &Expr) -> i32 {
    match expr {
        Expr::Command(cmd) => execute_single_command(cmd),
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

fn execute_single_command(cmd: &CommandExpr) -> i32 {
    if cmd.argv.is_empty() {
        return 0;
    }

    let mut command = Command::new(&cmd.argv[0]);
    command.args(&cmd.argv[1..]);

    if let Some((ref path, append)) = cmd.stdout {
        let file = if append {
            std::fs::OpenOptions::new().append(true).create(true).open(path)
        } else {
            std::fs::File::create(path)
        };
        if let Ok(f) = file {
            command.stdout(f);
        }
    }

    if let Some(ref path) = cmd.stderr {
        if let Ok(f) = std::fs::File::create(path) {
            command.stderr(f);
        }
    }

    if cmd.stderr_pipe {
        command.stderr(Stdio::piped()); // stderr をパイプに流す
    }

    command.status().ok().and_then(|s| s.code()).unwrap_or(1)
}

fn execute_pipeline(commands: &[CommandExpr]) -> i32 {
    if commands.is_empty() {
        return 0;
    }

    let mut processes = Vec::new();
    let mut prev_stdout = None;

    for (i, cmd) in commands.iter().enumerate() {
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
            if let Ok(f) = std::fs::File::create(path) {
                command.stderr(f);
            }
        }

        if cmd.stderr_pipe {
            command.stderr(Stdio::piped());
        }

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(_) => return 1,
        };

        if i != commands.len() - 1 {
            prev_stdout = child.stdout.take();
        }

        processes.push(child);
    }

    let (last, rest) = processes.split_last_mut().unwrap();
    for p in rest {
        let _ = p.wait();
    }

    last.wait().ok().and_then(|s| s.code()).unwrap_or(1)
}