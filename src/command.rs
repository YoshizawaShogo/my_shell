// command.rs
use std::process::{Command, Stdio};

#[derive(Debug)]
enum Expr {
    Command(CommandExpr),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Pipe(Vec<CommandExpr>),
}

#[derive(Debug)]
struct CommandExpr {
    argv: Vec<String>,
    stdout: Option<(String, bool)>, // (filename, append?)
    stderr: Option<String>,         // 2> or 2| target
    stderr_pipe: bool,              // true if 2| (pipe stderr)
}

pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = vec![];
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(&ch) = chars.peek() {
        match ch {
            '\'' => {
                chars.next();
                if in_single {
                    in_single = false;
                } else if !in_double {
                    in_single = true;
                } else {
                    current.push(ch);
                }
            }
            '"' => {
                chars.next();
                if in_double {
                    in_double = false;
                } else if !in_single {
                    in_double = true;
                } else {
                    current.push(ch);
                }
            }
            ' ' if !in_single && !in_double => {
                chars.next();
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            '&' | '|' | '>' => {
                chars.next();
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                if let Some(&next) = chars.peek() {
                    let combo = format!("{}{}", ch, next);
                    match combo.as_str() {
                        "&&" | "||" | ">>" | "&>" => {
                            chars.next();
                            tokens.push(combo);
                            continue;
                        }
                        _ => {}
                    }
                }
                tokens.push(ch.to_string());
            }
            '2' if !in_single && !in_double => {
                chars.next();
                if let Some(&next) = chars.peek() {
                    if next == '>' || next == '|' {
                        chars.next();
                        tokens.push(format!("2{}", next));
                        continue;
                    }
                }
                current.push('2');
            }
            _ => {
                current.push(ch);
                chars.next();
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn parse(tokens: &[String]) -> (Expr, usize) {
    parse_expr(tokens, 0)
}

fn parse_expr(tokens: &[String], mut i: usize) -> (Expr, usize) {
    let mut lhs = parse_pipe(tokens, &mut i);
    while i < tokens.len() {
        match tokens[i].as_str() {
            "&&" => {
                i += 1;
                let rhs = parse_pipe(tokens, &mut i);
                lhs = Expr::And(Box::new(lhs), Box::new(rhs));
            }
            "||" => {
                i += 1;
                let rhs = parse_pipe(tokens, &mut i);
                lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
            }
            _ => break,
        }
    }
    (lhs, i)
}

fn parse_pipe(tokens: &[String], i: &mut usize) -> Expr {
    let mut commands = vec![parse_command(tokens, i)];
    while *i < tokens.len() && tokens[*i] == "|" {
        *i += 1;
        commands.push(parse_command(tokens, i));
    }
    if commands.len() == 1 {
        Expr::Command(commands.remove(0))
    } else {
        Expr::Pipe(commands)
    }
}

fn parse_command(tokens: &[String], i: &mut usize) -> CommandExpr {
    let mut argv = Vec::new();
    let mut stdout = None;
    let mut stderr = None;
    let mut stderr_pipe = false;

    while *i < tokens.len() {
        match tokens[*i].as_str() {
            ">" | ">>" | "&>" => {
                let append = tokens[*i] == ">>";
                *i += 1;
                if *i < tokens.len() {
                    stdout = Some((tokens[*i].clone(), append));
                    *i += 1;
                }
            }
            "2>" => {
                *i += 1;
                if *i < tokens.len() {
                    stderr = Some(tokens[*i].clone());
                    *i += 1;
                }
            }
            "2|" => {
                *i += 1;
                stderr_pipe = true;
            }
            "|" | "&&" | "||" => break,
            _ => {
                argv.push(tokens[*i].clone());
                *i += 1;
            }
        }
    }

    CommandExpr { argv, stdout, stderr, stderr_pipe }
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

pub fn run(input: &str) -> i32 {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return 0;
    }
    let (expr, _) = parse(&tokens);
    execute(&expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_and_run_basic() {
        let result = run("echo hello > test_stdout.txt");
        assert_eq!(result, 0);
        let content = std::fs::read_to_string("test_stdout.txt").unwrap();
        assert!(content.contains("hello"));
        let _ = std::fs::remove_file("test_stdout.txt");
    }

    #[test]
    fn test_logical_and_or() {
        let result = run("false || echo yes > or_result.txt");
        assert_eq!(result, 0);
        let content = std::fs::read_to_string("or_result.txt").unwrap();
        assert!(content.contains("yes"));
        let _ = std::fs::remove_file("or_result.txt");
    }

    #[test]
    fn test_pipe_and_stderr_pipe() {
        let result = run("ls non_existing 2| grep foo");
        assert_ne!(result, 0); // expected non-zero since no stderr is captured
    }

    #[test]
    fn test_tokenize_quotes() {
        let tokens = tokenize("echo 'hello world' && ls");
        assert_eq!(
            tokens,
            vec!["echo", "hello world", "&&", "ls"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_redirections() {
        let tokens = tokenize("echo test > out.txt 2> err.txt");
        assert_eq!(
            tokens,
            vec!["echo", "test", ">", "out.txt", "2>", "err.txt"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        let (expr, _) = parse(&tokens);
        match expr {
            Expr::Command(cmd) => {
                assert_eq!(cmd.argv, vec!["echo", "test"]);
                assert_eq!(cmd.stdout, Some(("out.txt".to_string(), false)));
                assert_eq!(cmd.stderr, Some("err.txt".to_string()));
            }
            _ => panic!("Expected Command variant"),
        }
    }
}
