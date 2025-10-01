use crate::{command::tokenize::Token, expansion::Aliases};

#[derive(Debug)]
pub(crate) enum Expr {
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Pipe(Vec<CommandExpr>),
}

impl Expr {
    pub(crate) fn last_cmd_expr(&self) -> CommandExpr {
        let mut expr = self;
        loop {
            match expr {
                Expr::And(_, b) => expr = b,
                Expr::Or(_, b) => expr = b,
                Expr::Pipe(a) => {
                    return a.last().unwrap().clone();
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Redirection {
    /// パイプでつなぐ
    Pipe,
    /// 親プロセスと同じように標準入出力を使う
    Inherit,
    /// ファイルに出力 (path, append?)
    File { path: String, append: bool },
}

#[derive(Debug, Clone)]
pub(crate) struct CommandExpr {
    pub(crate) cmd_name: String,
    pub(crate) args: Vec<String>,
    pub(crate) stdout: Redirection,
    pub(crate) stderr: Redirection,
}

pub(crate) fn parse(tokens: &[Token], aliases: &Aliases) -> (Expr, usize) {
    parse_expr(tokens, 0, aliases)
}

fn parse_expr(tokens: &[Token], mut i: usize, aliases: &Aliases) -> (Expr, usize) {
    let mut lhs = parse_pipe(tokens, &mut i, aliases);
    while i < tokens.len() {
        match &tokens[i] {
            Token::And => {
                i += 1;
                let rhs = parse_pipe(tokens, &mut i, aliases);
                lhs = Expr::And(Box::new(lhs), Box::new(rhs));
            }
            Token::Or => {
                i += 1;
                let rhs = parse_pipe(tokens, &mut i, aliases);
                lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
            }
            _ => break,
        }
    }
    (lhs, i)
}

fn parse_pipe(tokens: &[Token], i: &mut usize, aliases: &Aliases) -> Expr {
    let mut commands = vec![parse_command(tokens, i, aliases)];
    while *i < tokens.len() && matches!(tokens[*i], Token::Pipe) {
        *i += 1;
        commands.push(parse_command(tokens, i, aliases));
    }
    if commands.len() == 1 {
        Expr::Pipe(commands)
    } else {
        Expr::Pipe(commands)
    }
}

fn parse_command(tokens: &[Token], i: &mut usize, aliases: &Aliases) -> CommandExpr {
    let mut cmd_name = match &tokens[*i] {
        Token::Word(x) => x.clone(),
        Token::LiteralWord(x) => x.clone(),
        _ => unreachable!(),
    };
    *i += 1;
    let mut argv = Vec::new();
    if aliases.contains_key(&cmd_name) {
        let expnaded = aliases.get(&cmd_name).unwrap();
        let mut expanded = expnaded.split_whitespace();
        cmd_name = expanded.next().unwrap().to_string();
        while let Some(x) = expanded.next() {
            argv.push(x.to_string());
        }
    }

    let mut stdout = Redirection::Inherit;
    let mut stderr = Redirection::Inherit;

    while *i < tokens.len() {
        match &tokens[*i] {
            Token::RedirectOut | Token::RedirectAppend => {
                let append = tokens[*i] == Token::RedirectAppend;
                *i += 1;
                if *i < tokens.len() {
                    if let Token::Word(filename) = &tokens[*i] {
                        stdout = Redirection::File {
                            path: filename.clone(),
                            append: append,
                        };
                        *i += 1;
                    }
                }
            }
            Token::RedirectBoth | Token::RedirectBothAppend => {
                let append = tokens[*i] == Token::RedirectBothAppend;
                *i += 1;
                if *i < tokens.len() {
                    if let Token::Word(filename) = &tokens[*i] {
                        stdout = Redirection::File {
                            path: filename.clone(),
                            append: append,
                        };
                        stderr = Redirection::File {
                            path: filename.clone(),
                            append: append,
                        };
                        *i += 1;
                    }
                }
            }
            Token::RedirectErr | Token::RedirectErrAppend => {
                let append = tokens[*i] == Token::RedirectErrAppend;
                *i += 1;
                if *i < tokens.len() {
                    if let Token::Word(filename) = &tokens[*i] {
                        stderr = Redirection::File {
                            path: filename.clone(),
                            append: append,
                        };
                        *i += 1;
                    }
                }
            }
            Token::Pipe => {
                stdout = Redirection::Pipe;
                break;
            }
            Token::PipeErr => {
                *i += 1;
                stderr = Redirection::Pipe;
            }
            Token::PipeBoth => {
                *i += 1;
                stdout = Redirection::Pipe;
                stderr = Redirection::Pipe;
            }
            Token::And | Token::Or => break,
            Token::Word(word) => {
                argv.push(word.clone());
                *i += 1;
            }
            Token::LiteralWord(word) => {
                argv.push(word.clone());
                *i += 1;
            }
        }
    }
    CommandExpr {
        cmd_name,
        args: argv,
        stdout,
        stderr,
    }
}
