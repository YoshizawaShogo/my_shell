mod key;
mod stream;

use crate::error::Result;
use crate::ui::action::key::Modifier;
use key::{Key, parse_keys};
use libc::read;
use std::{cmp::Ordering, io};
use stream::drain_burst;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Char(char),
    Ctrl(char),
    PreCmd,
    NextCmd,
    Up,
    Down,
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
    None,
}

trait Keymap {
    fn map(&self, key: Key) -> Action;
}

pub enum Mode {
    LineEdit,
    Completion,
}

struct LineEditKeymap;
impl Keymap for LineEditKeymap {
    fn map(&self, key: Key) -> Action {
        match key {
            Key::Char(' ', Modifier { .. }) => Action::Space,
            Key::Backspace(_) | Key::Char('h', Modifier { ctrl: true, .. }) => Action::BackSpace,
            Key::Enter(_) | Key::Char('j', Modifier { ctrl: true, .. }) => Action::Enter,
            Key::Char('l', Modifier { ctrl: true, .. }) => Action::Clear,
            Key::Char('w', Modifier { ctrl: true, .. }) => Action::DeleteWord,
            Key::Tab(_) => Action::Tab,
            Key::ArrowLeft(_) => Action::Left,
            Key::ArrowRight(_) | Key::Char('f', Modifier { ctrl: true, .. }) => Action::Right,
            Key::ArrowUp(_) => Action::PreCmd,
            Key::ArrowDown(_) => Action::NextCmd,
            Key::Home(_) | Key::Char('a', Modifier { ctrl: true, .. }) => Action::Home,
            Key::End(_) | Key::Char('e', Modifier { ctrl: true, .. }) => Action::End,
            Key::Delete(_) => Action::Delete,

            // default
            Key::Char(
                c,
                Modifier {
                    alt: false,
                    ctrl: false,
                    ..
                },
            ) => Action::Char(c),
            Key::Char(c, Modifier { ctrl: true, .. }) => Action::Ctrl(c),
            _ => Action::None,
        }
    }
}

struct CompletionKeymap;
impl Keymap for CompletionKeymap {
    fn map(&self, key: Key) -> Action {
        match key {
            Key::ArrowLeft(_) => Action::Left,
            Key::ArrowRight(_) | Key::Tab(_) => Action::Right,
            Key::ArrowUp(_) => Action::Up,
            Key::ArrowDown(_) => Action::Down,
            Key::Char('c', Modifier { ctrl: true, .. }) => Action::Char('c'),
            _ => Action::None,
        }
    }
}

fn current_keymap(mode: &Mode) -> &'static dyn Keymap {
    match mode {
        Mode::LineEdit => &LineEditKeymap,
        Mode::Completion => &CompletionKeymap,
    }
}

pub fn wait_actions(mode: &Mode, timeout_ms: i64) -> Result<Vec<Action>> {
    let keys = wait_keys(timeout_ms)?;
    let mapper = current_keymap(mode);
    let actions = keys.into_iter().map(|x| mapper.map(x));
    Ok(actions.collect())
}

fn wait_keys(timeout_ms: i64) -> io::Result<Vec<Key>> {
    loop {
        let mut b: u8 = 0;
        let n = unsafe { read(0, &mut b as *mut u8 as *mut _, 1) };

        match n.cmp(&0) {
            Ordering::Less => {
                let err = io::Error::last_os_error();
                // シグナル割り込みはリトライ
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }
            Ordering::Equal => {
                // EOF（例: Ctrl-D が行頭で押されたとき等）
                return Ok(Vec::new());
            }
            Ordering::Greater => {}
        }

        let burst = drain_burst(b, timeout_ms)?;
        return Ok(parse_keys(&burst));
    }
}
