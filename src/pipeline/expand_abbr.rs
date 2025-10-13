use super::tokenize::{QuoteKind, Token, tokenize};
use crate::shell::Shell;

/// abbr 展開（最後が**Unquoted** な Word の時のみ）
pub fn expand_abbr(mut tokens: Vec<Token>, shell: &Shell) -> Option<Vec<Token>> {
    let Some(last_idx) = tokens
        .iter()
        .rposition(|t| matches!(t, Token::Word(_, QuoteKind::None)))
    else {
        return None;
    };

    if !is_command_position(&tokens, last_idx) {
        return None;
    }

    if let Token::Word(word, QuoteKind::None) = &tokens[last_idx]
        && let Some(expansion) = { shell.abbrs.get(word).cloned() }
    {
        let repl = tokenize(&expansion);
        tokens.splice(last_idx..=last_idx, repl);
    }

    Some(tokens)
}

pub(super) fn is_command_position(tokens: &[Token], idx_of_current_word: usize) -> bool {
    if idx_of_current_word == 0 {
        return true;
    }
    match tokens[idx_of_current_word - 1] {
        Token::Pipe         // |
        | Token::PipeErr    // 2|
        | Token::PipeBoth   // &|
        | Token::And        // &&
        | Token::Or         // ||
        => true,
        Token::Delimiter => is_command_position(tokens, idx_of_current_word-1),
        _ => false,
    }
}
