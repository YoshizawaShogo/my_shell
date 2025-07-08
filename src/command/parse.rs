use crate::command::tokenize::Token;

#[derive(Debug)]
pub enum Expr {
    Command(CommandExpr),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Pipe(Vec<CommandExpr>),
}

#[derive(Debug)]
pub struct CommandExpr {
    pub(crate) argv: Vec<String>,
    pub(crate) stdout: Option<(String, bool)>,  // (filename, append?)
    pub(crate) stderr: Option<String>,         // 2> or 2| target
    pub(crate) stderr_pipe: bool,              // true if 2| (pipe stderr)
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