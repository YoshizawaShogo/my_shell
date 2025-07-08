use std::{env, fs, os::unix::fs::PermissionsExt, path::Path};

use crate::command::tokenize::Token;

pub fn command_exists(cmd: &str) -> bool {
    // 絶対パスや相対パスが指定されている場合は直接確認
    if cmd.contains('/') {
        return Path::new(cmd).is_file() && fs::metadata(cmd).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false);
    }

    // $PATH 環境変数にあるディレクトリを調べる
    if let Some(paths) = env::var_os("PATH") {
        for path in env::split_paths(&paths) {
            let full_path = path.join(cmd);
            if full_path.is_file() && fs::metadata(&full_path).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false) {
                return true;
            }
        }
    }
    false
}

pub fn validate_command(tokens: &[Token]) -> Option<String> {
    if let Some(Token::Word(first)) = tokens.first() {
        if !first.starts_with(|c: char| c == '-' || c == '/') && !command_exists(first) {
            return Some(format!("command not found: {}", first));
        }
    }
    None
}