// command.rs
use std::{env, fs, os::unix::fs::PermissionsExt, path::Path, process::{Command, Stdio}};

use crate::term_mode::{set_origin_term, set_raw_term};

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Word(String),           // 通常の文字列
    And,                    // &&
    Or,                     // ||
    RedirectOut,            // >
    RedirectBoth,           // &>
    RedirectErr,            // 2>
    RedirectAppend,         // >>
    RedirectBothAppend,     // &>>
    RedirectErrAppend,      // 2>>
    Pipe,                   // |
    PipeErr,                // 2|
    PipeBoth,               // &|
}

fn tokenize_a_word(word: &str) -> Token {
    match word {
        "&&" => Token::And,
        "||" => Token::Or,
        ">" => Token::RedirectOut,
        "&>" => Token::RedirectBoth,
        "2>" => Token::RedirectErr,
        ">>" => Token::RedirectAppend,
        "&>>" => Token::RedirectBothAppend,
        "2>>" => Token::RedirectErrAppend,
        "|" => Token::Pipe,
        "2|" => Token::PipeErr,
        "&|" => Token::PipeBoth,
        _ => Token::Word(word.to_string()),
    }
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = vec![];
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next_ch) = chars.next() {
                current.push('\\');
                current.push(next_ch);
            }
            continue;
        }
        match ch {
            '\'' if !in_double => {
                if in_single {
                    in_single = false;
                    tokens.push(tokenize_a_word(&current));
                    current.clear();
                } else {
                    in_single = true;
                }
            }
            '\"' if !in_single => {
                if in_double {
                    in_double = false;
                    tokens.push(tokenize_a_word(&current));
                    current.clear();
                } else {
                    in_double = true;
                }
            }
            ' ' if !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(tokenize_a_word(&current));
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(tokenize_a_word(&current));
    }
    tokens
}

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

pub fn command_exists(cmd: &str) -> bool {
    // 絶対パスや相対パスが指定されている場合は直接確認
    if cmd.contains('/') {
        return Path::new(cmd).is_file() && fs::metadata(cmd).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false);
    }

    // $PATH 環境変数にあるディレクトリを調べる
    if let Some(paths) = env::var_os("PATH") {
        for path in env::split_paths(&paths) {
            let full_path = path.join(cmd);
            if full_path.is_file() && fs::metadata(&full_path).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false) {
                return true;
            }
        }
    }
    false
}

pub fn validate_command(tokens: &[Token]) -> Option<String> {
    if let Some(Token::Word(first)) = tokens.first() {
        if !first.starts_with(|c: char| c == '-' || c == '/') && !command_exists(first) {
            return Some(format!("command not found: {}", first));
        }
    }
    None
}

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

fn parse(tokens: &[Token]) -> (Expr, usize) {
    parse_expr(tokens, 0)
}

fn parse_expr(tokens: &[Token], mut i: usize) -> (Expr, usize) {
    let mut lhs = parse_pipe(tokens, &mut i);
    while i < tokens.len() {
        match &tokens[i] {
            Token::And => {
                i += 1;
                let rhs = parse_pipe(tokens, &mut i);
                lhs = Expr::And(Box::new(lhs), Box::new(rhs));
            }
            Token::Or => {
                i += 1;
                let rhs = parse_pipe(tokens, &mut i);
                lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
            }
            _ => break,
        }
    }
    (lhs, i)
}

fn parse_pipe(tokens: &[Token], i: &mut usize) -> Expr {
    let mut commands = vec![parse_command(tokens, i)];
    while *i < tokens.len() && matches!(tokens[*i], Token::Pipe) {
        *i += 1;
        commands.push(parse_command(tokens, i));
    }
    if commands.len() == 1 {
        Expr::Command(commands.remove(0))
    } else {
        Expr::Pipe(commands)
    }
}

fn parse_command(tokens: &[Token], i: &mut usize) -> CommandExpr {
    let mut argv = Vec::new();
    let mut stdout = None;
    let mut stderr = None;
    let mut stderr_pipe = false;

    while *i < tokens.len() {
        match &tokens[*i] {
            Token::RedirectOut | Token::RedirectAppend | Token::RedirectBoth | Token::RedirectBothAppend => {
                let append = matches!(tokens[*i], Token::RedirectAppend | Token::RedirectBothAppend);
                *i += 1;
                if *i < tokens.len() {
                    if let Token::Word(filename) = &tokens[*i] {
                        stdout = Some((filename.clone(), append));
                        *i += 1;
                    }
                }
            }
            Token::RedirectErr | Token::RedirectErrAppend => {
                *i += 1;
                if *i < tokens.len() {
                    if let Token::Word(filename) = &tokens[*i] {
                        stderr = Some(filename.clone());
                        *i += 1;
                    }
                }
            }
            Token::PipeErr => {
                *i += 1;
                stderr_pipe = true;
            }
            Token::PipeBoth => {
                // TODO: Handle &| (pipe both stdout and stderr)
                *i += 1;
            }
            Token::Pipe | Token::And | Token::Or => break,
            Token::Word(word) => {
                argv.push(word.clone());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_words() {
        assert_eq!(tokenize("echo hello"), vec![
            Token::Word("echo".into()),
            Token::Word("hello".into()),
        ]);
    }

    #[test]
    fn test_quoted_words() {
        assert_eq!(tokenize("\"\\\"a'a\\\"\""), vec![Token::Word("\\\"a'a\\\"".into())]);
        assert_eq!(tokenize("\"a'a\""), vec![Token::Word("a'a".into())]);
    }

    #[test]
    fn test_and_or() {
        assert_eq!(tokenize("a && b || c"), vec![
            Token::Word("a".into()),
            Token::And,
            Token::Word("b".into()),
            Token::Or,
            Token::Word("c".into()),
        ]);
    }

    #[test]
    fn test_redirects() {
        assert_eq!(tokenize("a > b"), vec![Token::Word("a".into()), Token::RedirectOut, Token::Word("b".into())]);
        assert_eq!(tokenize("a >> b"), vec![Token::Word("a".into()), Token::RedirectAppend, Token::Word("b".into())]);
        assert_eq!(tokenize("a &> b"), vec![Token::Word("a".into()), Token::RedirectBoth, Token::Word("b".into())]);
        assert_eq!(tokenize("a &>> b"), vec![Token::Word("a".into()), Token::RedirectBothAppend, Token::Word("b".into())]);
        assert_eq!(tokenize("a 2> b"), vec![Token::Word("a".into()), Token::RedirectErr, Token::Word("b".into())]);
        assert_eq!(tokenize("a 2>> b"), vec![Token::Word("a".into()), Token::RedirectErrAppend, Token::Word("b".into())]);
    }

    #[test]
    fn test_pipes() {
        assert_eq!(tokenize("a | b"), vec![Token::Word("a".into()), Token::Pipe, Token::Word("b".into())]);
        assert_eq!(tokenize("a || b"), vec![Token::Word("a".into()), Token::Or, Token::Word("b".into())]);
        assert_eq!(tokenize("a &| b"), vec![Token::Word("a".into()), Token::PipeBoth, Token::Word("b".into())]);
        assert_eq!(tokenize("a 2| b"), vec![Token::Word("a".into()), Token::PipeErr, Token::Word("b".into())]);
    }

    #[test]
    fn test_mixed_tokens() {
        let input = "echo 'hello world' && ls -l | grep txt &> result.log";
        let expected = vec![
            Token::Word("echo".into()),
            Token::Word("hello world".into()),
            Token::And,
            Token::Word("ls".into()),
            Token::Word("-l".into()),
            Token::Pipe,
            Token::Word("grep".into()),
            Token::Word("txt".into()),
            Token::RedirectBoth,
            Token::Word("result.log".into()),
        ];
        assert_eq!(tokenize(input), expected);
    }

    #[test]
    fn test_no_space_operators() {
        let input = "a && b || c | d &> e &>> f 2> g 2>> h 2| i";
        let expected = vec![
            Token::Word("a".into()),
            Token::And,
            Token::Word("b".into()),
            Token::Or,
            Token::Word("c".into()),
            Token::Pipe,
            Token::Word("d".into()),
            Token::RedirectBoth,
            Token::Word("e".into()),
            Token::RedirectBothAppend,
            Token::Word("f".into()),
            Token::RedirectErr,
            Token::Word("g".into()),
            Token::RedirectErrAppend,
            Token::Word("h".into()),
            Token::PipeErr,
            Token::Word("i".into()),
        ];
        assert_eq!(tokenize(input), expected);
    }
}