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
    stderr: Option<String>,         // 2> filename
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
                    if next == ch {
                        chars.next();
                        tokens.push(format!("{}{}", ch, ch));
                        continue;
                    }
                }
                if ch == '>' && !tokens.is_empty() && tokens.last().unwrap() == "2" {
                    tokens.pop();
                    tokens.push("2>".to_string());
                } else {
                    tokens.push(ch.to_string());
                }
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

    while *i < tokens.len() {
        match tokens[*i].as_str() {
            ">" | ">>" => {
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
            "|" | "&&" | "||" => break,
            _ => {
                argv.push(tokens[*i].clone());
                *i += 1;
            }
        }
    }

    CommandExpr { argv, stdout, stderr }
}
pub fn run(input: &str) -> i32 {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return 0;
    }

    let (expr, _) = parse(&tokens);
    execute(&expr)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_quotes_and_redirects() {
        let input = r#"echo "hello world" >> out.txt 2> err.log"#;
        let tokens = tokenize(input);
        assert_eq!(
            tokens,
            vec![
                "echo", "hello world", ">>", "out.txt", "2>", "err.log"
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_single_command() {
        let tokens = tokenize(r#"echo hello > out.txt 2> err.log"#);
        let (expr, _) = parse(&tokens);
        if let Expr::Command(cmd) = expr {
            assert_eq!(cmd.argv, vec!["echo", "hello"]);
            assert_eq!(cmd.stdout, Some(("out.txt".to_string(), false)));
            assert_eq!(cmd.stderr, Some("err.log".to_string()));
        } else {
            panic!("Expected Command variant");
        }
    }

    #[test]
    fn test_parse_pipe() {
        let tokens = tokenize(r#"cat a | grep foo | sort"#);
        let (expr, _) = parse(&tokens);
        if let Expr::Pipe(cmds) = expr {
            assert_eq!(cmds.len(), 3);
            assert_eq!(cmds[0].argv, vec!["cat", "a"]);
            assert_eq!(cmds[1].argv, vec!["grep", "foo"]);
            assert_eq!(cmds[2].argv, vec!["sort"]);
        } else {
            panic!("Expected Pipe variant");
        }
    }

    #[test]
    fn test_parse_logical_operators() {
        let tokens = tokenize(r#"false || echo ok && echo done"#);
        let (expr, _) = parse(&tokens);
        match expr {
            Expr::And(lhs, rhs) => {
                assert!(matches!(*rhs, Expr::Command(_)));
                match &*lhs {
                    Expr::Or(l, r) => {
                        assert!(matches!(**l, Expr::Command(_)));
                        assert!(matches!(**r, Expr::Command(_)));
                    }
                    _ => panic!("Expected Or"),
                }
            }
            _ => panic!("Expected And at top level"),
        }
    }
}
