use crate::input::key::{Key, Modifier};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum KeyFunction {
    Char(char),
    Ctrl(char),
    PreCmd,     // 履歴: 前のコマンド（↑ / PageUp）
    NextCmd,    // 履歴: 次のコマンド（↓ / PageDown）
    Left,       // ←
    Right,      // →
    Tab,        // Tab
    Home,       // Home
    End,        // End
    Enter,      // Enter / Return
    BackSpace,  // Backspace
    Delete,     // Delete
    Clear,      // 画面クリア（既定: Ctrl-L）
    DeleteWord, // 単語削除（既定: Alt-Backspace / Ctrl-W）
}

impl Key {
    pub fn function(&self) -> Option<KeyFunction> {
        Some(match self {
            Key::Char(
                c,
                Modifier {
                    alt: false,
                    ctrl: false,
                    ..
                },
            ) => KeyFunction::Char(*c),
            Key::Backspace(_) | Key::Char('h', Modifier { ctrl: true, .. }) => {
                KeyFunction::BackSpace
            }
            Key::Enter(_) | Key::Char('j', Modifier { ctrl: true, .. }) => KeyFunction::Enter,
            Key::Char('l', Modifier { ctrl: true, .. }) => KeyFunction::Clear,
            Key::Char('w', Modifier { ctrl: true, .. }) => KeyFunction::DeleteWord,
            Key::Char(c, Modifier { ctrl: true, .. }) => KeyFunction::Ctrl(*c),
            Key::Tab(_) => KeyFunction::Tab,
            Key::ArrowLeft(_) => KeyFunction::Left,
            Key::ArrowRight(_) => KeyFunction::Right,
            Key::ArrowUp(_) => KeyFunction::PreCmd,
            Key::ArrowDown(_) => KeyFunction::NextCmd,
            Key::Home(_) => KeyFunction::Home,
            Key::End(_) => KeyFunction::End,
            Key::Delete(_) => KeyFunction::Delete,

            _ => return None,
        })
    }
}
