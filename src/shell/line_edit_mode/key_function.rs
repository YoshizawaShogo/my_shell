use crate::input::key::{Key, Modifier};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum KeyFunction {
    Char(char),
    Ctrl(char),
    PreCmd,
    NextCmd,
    Left,
    Right,
    Tab,
    Home,
    End,
    Enter,
    BackSpace,
    Delete,
    Clear,
    DeleteWord,
    Space,
}

impl Key {
    pub(super) fn edit_function(&self) -> Option<KeyFunction> {
        Some(match self {
            Key::Char(' ', Modifier { .. }) => KeyFunction::Space,
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
