use std::mem;

use crate::expansion::Aliases;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Token {
    Word(String),        // 文字列"", (クウォート無しも)
    LiteralWord(String), // 文字列''
    And,                 // &&
    Or,                  // ||
    RedirectOut,         // >
    RedirectBoth,        // &>
    RedirectErr,         // 2>
    RedirectAppend,      // >>
    RedirectBothAppend,  // &>>
    RedirectErrAppend,   // 2>>
    Pipe,                // |
    PipeErr,             // 2|
    PipeBoth,            // &|
}

impl<T: AsRef<str>> From<T> for Token {
    fn from(word: T) -> Token {
        match word.as_ref() {
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
            other => Token::Word(other.to_string()),
        }
    }
}

pub(crate) struct Tokens {
    inner: Vec<Token>,
}

impl<T: AsRef<str>> From<T> for Tokens {
    fn from(value: T) -> Self {
        Self {
            inner: tokenize(value.as_ref()),
        }
    }
}

impl Tokens {
    pub(crate) fn parse(self, aliases: &Aliases) -> crate::command::parse::Expr {
        crate::command::parse::parse(&self.inner, aliases).0
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

pub(crate) fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = vec![];
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(next_ch) = chars.next() {
                    // current.push('\\');
                    current.push(next_ch);
                }
            }
            '\'' if !in_double => {
                if in_single {
                    in_single = false;
                    tokens.push(Token::LiteralWord(mem::take(&mut current)));
                    current.clear();
                } else {
                    in_single = true;
                }
            }
            '\"' if !in_single => {
                if in_double {
                    in_double = false;
                    tokens.push(mem::take(&mut current).into());
                } else {
                    in_double = true;
                }
            }
            ' ' if !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(mem::take(&mut current).into());
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current.into());
    }
    tokens
}
