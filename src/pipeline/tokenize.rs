use std::mem;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Word(String, QuoteKind),
    And,                // &&
    Or,                 // ||
    RedirectOut,        // >
    RedirectBoth,       // &>
    RedirectErr,        // 2>
    RedirectAppend,     // >>
    RedirectBothAppend, // &>>
    RedirectErrAppend,  // 2>>
    Pipe,               // |
    PipeErr,            // 2|
    PipeBoth,           // &|
    Delimiter,          // 区切り文字(token間のspaceを明示)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteKind {
    None,
    Single,
    Double,
    Variable,
    Tilde,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    use std::iter::Peekable;
    use std::str::Chars;

    fn peek2(it: &Peekable<Chars<'_>>) -> (Option<char>, Option<char>) {
        let mut it2 = it.clone();
        (it2.next(), it2.next())
    }

    fn match_operator(ch: char, it: &Peekable<Chars<'_>>) -> Option<(Token, usize)> {
        let (p1, p2) = peek2(it);
        match (ch, p1, p2) {
            // 3文字
            ('&', Some('>'), Some('>')) => Some((Token::RedirectBothAppend, 3)),
            ('2', Some('>'), Some('>')) => Some((Token::RedirectErrAppend, 3)),
            // 2文字
            ('&', Some('&'), _) => Some((Token::And, 2)),
            ('|', Some('|'), _) => Some((Token::Or, 2)),
            ('>', Some('>'), _) => Some((Token::RedirectAppend, 2)),
            ('&', Some('>'), _) => Some((Token::RedirectBoth, 2)),
            ('2', Some('>'), _) => Some((Token::RedirectErr, 2)),
            ('&', Some('|'), _) => Some((Token::PipeBoth, 2)),
            ('2', Some('|'), _) => Some((Token::PipeErr, 2)),
            // 1文字
            ('|', _, _) => Some((Token::Pipe, 1)),
            ('>', _, _) => Some((Token::RedirectOut, 1)),
            _ => None,
        }
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_variable = false;
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = chars.next() {
        if in_variable {
            match ch {
                'a'..'z' | 'A'..'Z' | '_' => {
                    current.push(ch);
                    continue;
                }
                _ => {
                    in_variable = false;
                    tokens.push(Token::Word(mem::take(&mut current), QuoteKind::Variable));
                }
            }
        }

        if in_single {
            match ch {
                '\'' => {
                    in_single = false;
                    tokens.push(Token::Word(mem::take(&mut current), QuoteKind::Single));
                }
                _ => current.push(ch), // シングル内はリテラル
            }
            continue;
        }

        if in_double {
            match ch {
                '"' => {
                    in_double = false;
                    tokens.push(Token::Word(mem::take(&mut current), QuoteKind::Double));
                }
                '\\' => {
                    // 簡易: 次の1文字をそのまま取り込む（\" や \\ を保持）
                    if let Some(nc) = chars.next() {
                        current.push(nc);
                    } else {
                        current.push('\\');
                    }
                }
                '$' => {
                    in_variable = true;
                    tokens.push(Token::Word(mem::take(&mut current), QuoteKind::Double));
                }
                _ => current.push(ch),
            }
            continue;
        }

        // ── クォート外 ────────────────────────────────
        match ch {
            // 区切り（空白）
            ' ' | '\t' | '\n' | '\r' => {
                if !current.is_empty() {
                    tokens.push(Token::Word(mem::take(&mut current), QuoteKind::None));
                }
                tokens.push(Token::Delimiter);
            }
            // クォート開始
            '\'' => {
                if !current.is_empty() {
                    tokens.push(Token::Word(mem::take(&mut current), QuoteKind::None));
                }
                in_single = true;
            }
            '"' => {
                if !current.is_empty() {
                    tokens.push(Token::Word(mem::take(&mut current), QuoteKind::None));
                }
                in_double = true;
            }
            // バックスラッシュ（簡易）
            '\\' => {
                if let Some(nc) = chars.next() {
                    current.push(nc);
                } else {
                    current.push('\\');
                }
            }
            '$' => {
                if !current.is_empty() {
                    tokens.push(Token::Word(mem::take(&mut current), QuoteKind::None));
                }
                in_variable = true;
            }
            '~' => if current.is_empty() {
                tokens.push(Token::Word("~".to_string(), QuoteKind::Tilde));
            }
            // 演算子（最長一致）
            _ => {
                if let Some((tok, len)) = match_operator(ch, &chars) {
                    if !current.is_empty() {
                        tokens.push(Token::Word(mem::take(&mut current), QuoteKind::None));
                    }
                    for _ in 1..len {
                        chars.next();
                    }
                    tokens.push(tok);
                } else {
                    current.push(ch);
                }
            }
        }
    }

    if !current.is_empty() {
        tokens.push(Token::Word(current, QuoteKind::None));
    }

    tokens
}

pub fn tokens_to_string(tokens: &[Token]) -> String {
    fn quote_single(s: &str) -> String {
        // ' を含む場合は:  'foo'\''bar'
        if s.contains('\'') {
            let mut out = String::from("'");
            for ch in s.chars() {
                if ch == '\'' {
                    out.push_str("'\\''");
                } else {
                    out.push(ch);
                }
            }
            out.push('\'');
            out
        } else {
            format!("'{}'", s)
        }
    }

    fn quote_double(s: &str) -> String {
        // 簡易: " と \ をエスケープ（必要なら $ や ` も）
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    }

    let mut parts = Vec::with_capacity(tokens.len());
    for t in tokens {
        let s = match t {
            Token::Word(w, QuoteKind::None) => w.clone(),
            Token::Word(w, QuoteKind::Single) => quote_single(w),
            Token::Word(w, QuoteKind::Double) => quote_double(w),
            Token::Word(w, QuoteKind::Variable) => "$".to_string() + w,
            Token::Word(w, QuoteKind::Tilde) => w.to_string(),
            Token::And => "&&".to_string(),
            Token::Or => "||".to_string(),
            Token::RedirectOut => ">".to_string(),
            Token::RedirectBoth => "&>".to_string(),
            Token::RedirectErr => "2>".to_string(),
            Token::RedirectAppend => ">>".to_string(),
            Token::RedirectBothAppend => "&>>".to_string(),
            Token::RedirectErrAppend => "2>>".to_string(),
            Token::Pipe => "|".to_string(),
            Token::PipeErr => "2|".to_string(),
            Token::PipeBoth => "&|".to_string(),
            Token::Delimiter => " ".to_string(),
        };
        parts.push(s);
    }
    parts.join("")
}
