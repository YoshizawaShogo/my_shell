use super::tokenize::{QuoteKind, Token};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub enum Segment {
    Unquoted(String),
    DoubleQuoted(String),
    SingleQuoted(String),
    Variable(String),
}

#[derive(Debug, Clone)]
pub struct WordNode {
    pub segments: Vec<Segment>,
}

impl WordNode {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn concat_text(&self) -> String {
        let mut s = String::new();
        for seg in &self.segments {
            match seg {
                Segment::Unquoted(t) | Segment::DoubleQuoted(t) | Segment::SingleQuoted(t) => {
                    s.push_str(t)
                }
                Segment::Variable(t) => {
                    s.push('$');
                    s.push_str(t);
                }
            }
        }
        s
    }
}

#[derive(Debug)]
pub enum Expr {
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Pipe(Vec<CommandExpr>),
}

impl Expr {
    pub fn last_cmd_expr(&self) -> CommandExpr {
        let mut expr = self;
        loop {
            match expr {
                Expr::And(_, b) => expr = b,
                Expr::Or(_, b) => expr = b,
                Expr::Pipe(a) => return a.last().unwrap().clone(),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandExpr {
    pub cmd_name: WordNode,
    pub args: Vec<WordNode>,
    pub stdout: Redirection,
    pub stderr: Redirection,
}

#[derive(Debug, Clone)]
pub enum Redirection {
    Pipe,
    Inherit,
    File { path: WordNode, append: bool },
}

pub fn parse(tokens: &[Token]) -> Result<Expr> {
    Ok(parse_expr(tokens, 0)?.0)
}

fn parse_expr(tokens: &[Token], mut i: usize) -> Result<(Expr, usize)> {
    let mut lhs = parse_pipe(tokens, &mut i)?;
    while let Some(token) = skip_delimiter_get(tokens, &mut i) {
        match token {
            Token::And => {
                i += 1;
                let rhs = parse_pipe(tokens, &mut i)?;
                lhs = Expr::And(Box::new(lhs), Box::new(rhs));
            }
            Token::Or => {
                i += 1;
                let rhs = parse_pipe(tokens, &mut i)?;
                lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
            }
            _ => break,
        }
    }
    Ok((lhs, i))
}

fn parse_pipe(tokens: &[Token], i: &mut usize) -> Result<Expr> {
    let mut commands = vec![parse_command(tokens, i)?];
    while let Some(token) = skip_delimiter_get(tokens, i) {
        if !matches!(token, Token::Pipe) {
            break;
        }
        *i += 1;
        commands.push(parse_command(tokens, i)?);
    }
    Ok(Expr::Pipe(commands))
}

/// 1 トークン＝1 WordNode（クォート種別を Segment に落とす）
fn parse_word_node(tokens: &[Token], i: &mut usize) -> Result<WordNode> {
    let mut node = WordNode::new();
    while let Some(token) = tokens.get(*i) {
        *i += 1;
        match token {
            Token::Word(s, QuoteKind::None) => node.segments.push(Segment::Unquoted(s.clone())),
            Token::Word(s, QuoteKind::Single) => {
                node.segments.push(Segment::SingleQuoted(s.clone()))
            }
            Token::Word(s, QuoteKind::Double) => {
                node.segments.push(Segment::DoubleQuoted(s.clone()))
            }
            Token::Word(s, QuoteKind::Variable) => node.segments.push(Segment::Variable(s.clone())),
            _ => break,
        }
    }
    Ok(node)
}

fn parse_command(tokens: &[Token], i: &mut usize) -> Result<CommandExpr> {
    // 先頭はコマンド名
    let cmd_name = match must_get(tokens, i)? {
        Token::Word(_, _) => parse_word_node(tokens, i)?,
        _ => unreachable!("command must start with a word"),
    };

    let mut args: Vec<WordNode> = Vec::new();
    let mut stdout = Redirection::Inherit;
    let mut stderr = Redirection::Inherit;

    while *i < tokens.len() {
        let Some(token) = skip_delimiter_get(tokens, i) else {
            break;
        };
        match token {
            // > / >>  (stdout)
            Token::RedirectOut | Token::RedirectAppend => {
                let append = matches!(must_get(tokens, i)?, Token::RedirectAppend);
                *i += 1;
                if matches!(must_get(tokens, i)?, Token::Word(_, _)) {
                    let path = parse_word_node(tokens, i)?;
                    stdout = Redirection::File { path, append };
                }
            }
            // &> / &>>  (stdout, stderr 両方)
            Token::RedirectBoth | Token::RedirectBothAppend => {
                let append = matches!(must_get(tokens, i)?, Token::RedirectBothAppend);
                *i += 1;
                if matches!(must_get(tokens, i)?, Token::Word(_, _)) {
                    let path = parse_word_node(tokens, i)?;
                    stdout = Redirection::File {
                        path: path.clone(),
                        append,
                    };
                    stderr = Redirection::File { path, append };
                }
            }
            // 2> / 2>>  (stderr)
            Token::RedirectErr | Token::RedirectErrAppend => {
                let append = matches!(must_get(tokens, i)?, Token::RedirectErrAppend);
                *i += 1;
                if matches!(must_get(tokens, i)?, Token::Word(_, _)) {
                    let path = parse_word_node(tokens, i)?;
                    stderr = Redirection::File { path, append };
                }
            }

            // パイプ境界や論理境界
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

            // 引数
            Token::Word(_, _) => {
                let arg = parse_word_node(tokens, i)?;
                args.push(arg);
            }
            Token::Delimiter => {
                *i += 1;
                continue;
            }
        }
    }

    Ok(CommandExpr {
        cmd_name,
        args,
        stdout,
        stderr,
    })
}

fn must_get<'a>(tokens: &'a [Token], i: &'a mut usize) -> Result<&'a Token> {
    match tokens.get(*i) {
        Some(x) => match x {
            Token::Delimiter => {
                *i += 1;
                must_get(tokens, i)
            }
            x => Ok(x),
        },
        None => Err(Error::StructureCollaps),
    }
}

fn skip_delimiter_get<'a>(tokens: &'a [Token], i: &'a mut usize) -> Option<&'a Token> {
    match tokens.get(*i) {
        Some(x) => match x {
            Token::Delimiter => {
                *i += 1;
                skip_delimiter_get(tokens, i)
            }
            x => Some(x),
        },
        None => None,
    }
}
