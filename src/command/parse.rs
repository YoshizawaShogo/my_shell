use crate::command::tokenize::Token;

#[derive(Debug)]
pub enum Expr {
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Pipe(Vec<CommandExpr>),
}

#[derive(Debug)]
pub enum Redirection {
    /// パイプでつなぐ
    Pipe,
    /// 親プロセスと同じように標準入出力を使う
    Inherit,
    /// ファイルに出力 (path, append?)
    File { path: String, append: bool },
}

#[derive(Debug)]
pub struct CommandExpr {
    pub(crate) argv:  Vec<String>,
    pub(crate) stdout: Redirection,
    pub(crate) stderr: Redirection,
}

pub fn parse(tokens: &[Token]) -> (Expr, usize) {
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
        Expr::Pipe(commands)
    } else {
        Expr::Pipe(commands)
    }
}

fn parse_command(tokens: &[Token], i: &mut usize) -> CommandExpr {
    let mut argv = Vec::new();
    let mut stdout = Redirection::Inherit;
    let mut stderr = Redirection::Inherit;

    while *i < tokens.len() {
        match &tokens[*i] {
            Token::RedirectOut | Token::RedirectAppend => {
                let append = tokens[*i] == Token::RedirectAppend;
                *i += 1;
                if *i < tokens.len() {
                    if let Token::Word(filename) = &tokens[*i] {
                        stdout = Redirection::File { path: filename.clone(), append: append };
                        *i += 1;
                    }
                }
            }
            Token::RedirectBoth | Token::RedirectBothAppend => {
                let append = tokens[*i] == Token::RedirectBothAppend;
                *i += 1;
                if *i < tokens.len() {
                    if let Token::Word(filename) = &tokens[*i] {
                        stdout = Redirection::File { path: filename.clone(), append: append };
                        stderr = Redirection::File { path: filename.clone(), append: append };
                        *i += 1;
                    }
                }
            }
            Token::RedirectErr | Token::RedirectErrAppend => {
                let append = tokens[*i] == Token::RedirectErrAppend;
                *i += 1;
                if *i < tokens.len() {
                    if let Token::Word(filename) = &tokens[*i] {
                        stderr = Redirection::File { path: filename.clone(), append: append };
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
        argv,
        stdout,
        stderr,
    }
}
