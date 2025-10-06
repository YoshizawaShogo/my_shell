use std::sync::{Arc, Mutex};

use crate::shell::{
    Shell,
    pipeline::tokenize::{QuoteKind, Token, tokenize},
};

/// alias 展開（コマンド先頭のみ / QuoteKind::None のみ）
/// コマンド先頭は文頭または `|`, `2|`, `&|`, `&&`, `||` の直後。
pub fn expand_aliases(mut tokens: Vec<Token>, shell: &Arc<Mutex<Shell>>) -> Vec<Token> {
    let mut at_cmd_head = true;
    let mut i = 0;

    while i < tokens.len() {
        if at_cmd_head {
            // コマンド先頭が Unquoted Word のときだけ alias 対象
            if let Token::Word(alias_name, QuoteKind::None) = &tokens[i] {
                let expansion_opt = {
                    let sh = shell.lock().unwrap();
                    sh.aliases.get(alias_name).cloned()
                };

                if let Some(expansion) = expansion_opt {
                    let expanded = tokenize(&expansion);
                    tokens.splice(i..i + 1, expanded);
                }
            }
        }

        // コマンド境界更新
        match tokens.get(i) {
            Some(Token::Pipe)
            | Some(Token::PipeErr)
            | Some(Token::PipeBoth)
            | Some(Token::And)
            | Some(Token::Or) => at_cmd_head = true,
            _ => at_cmd_head = false,
        }

        i += 1;
    }

    tokens
}

/// abbr 展開（最後が**Unquoted** な Word の時のみ）
pub fn expand_abbr(mut tokens: Vec<Token>, shell: &Arc<Mutex<Shell>>) -> Vec<Token> {
    use crate::shell::pipeline::tokenize::QuoteKind;

    let Some(last_idx) = tokens
        .iter()
        .rposition(|t| matches!(t, Token::Word(_, QuoteKind::None)))
    else {
        return tokens;
    };

    if let Token::Word(word, QuoteKind::None) = &tokens[last_idx] {
        if let Some(expansion) = {
            let sh = shell.lock().unwrap();
            sh.abbrs.get(word).cloned()
        } {
            let repl = tokenize(&expansion);
            tokens.splice(last_idx..=last_idx, repl);
        }
    }

    tokens
}
