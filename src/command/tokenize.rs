#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Word(String),        // 通常の文字列
    LiteralWord(String), // 通常の文字列
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
        match ch {
            '\\' => {
                if let Some(next_ch) = chars.next() {
                    current.push('\\');
                    current.push(next_ch);
                }
            }
            '\'' if !in_double => {
                if in_single {
                    in_single = false;
                    tokens.push(Token::LiteralWord(current.to_string()));
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
