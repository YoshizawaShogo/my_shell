use super::tokenize::{QuoteKind, Token, tokenize};
use crate::shell::Shell;

/// alias 展開（コマンド先頭のみ / QuoteKind::None のみ）
/// コマンド先頭は文頭または `|`, `2|`, `&|`, `&&`, `||` の直後。
pub fn expand_aliases(mut tokens: Vec<Token>, shell: &Shell) -> Vec<Token> {
    let mut at_cmd_head = true;
    let mut i = 0;

    while i < tokens.len() {
        if at_cmd_head {
            // コマンド先頭が Unquoted Word のときだけ alias 対象
            if let Token::Word(alias_name, QuoteKind::None) = &tokens[i] {
                let expansion_opt = { shell.aliases.get(alias_name).cloned() };

                if let Some(expansion) = expansion_opt {
                    let expanded = tokenize(&expansion);
                    tokens.splice(i..i + 1, expanded);
                }
            }
        }

        // コマンド境界更新
        at_cmd_head = super::expand_abbr::is_command_position(&tokens, i + 1);
        i += 1;
    }

    tokens
}
