#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Modifier {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}
impl Modifier {
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
    };
    pub const fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }
    pub const fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }
    pub const fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Key {
    Char(char, Modifier),

    Enter(Modifier),
    Backspace(Modifier),
    Tab(Modifier),
    Escape(Modifier),

    ArrowUp(Modifier),
    ArrowDown(Modifier),
    ArrowLeft(Modifier),
    ArrowRight(Modifier),

    Home(Modifier),
    End(Modifier),
    PageUp(Modifier),
    PageDown(Modifier),
    Insert(Modifier),
    Delete(Modifier),

    F(u8, Modifier),

    Unknown,
}

pub fn parse_keys(bytes: &[u8]) -> Vec<Key> {
    let mut out = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        // ESC 始まり（CSI / SS3 / Alt）
        if b == 0x1b {
            // CSI: ESC [
            if i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                let (opt_key, used) = parse_csi(&bytes[i..]);
                if let Some(k) = opt_key {
                    out.push(k);
                    i += used;
                    continue;
                }
            }
            // SS3: ESC O ・・・ Home/End/矢印/F1..F4（修飾なし）
            if i + 2 < bytes.len() && bytes[i + 1] == b'O' {
                let k = match bytes[i + 2] {
                    b'H' => Some(Key::Home(Modifier::NONE)),
                    b'F' => Some(Key::End(Modifier::NONE)),
                    b'A' => Some(Key::ArrowUp(Modifier::NONE)),
                    b'B' => Some(Key::ArrowDown(Modifier::NONE)),
                    b'C' => Some(Key::ArrowRight(Modifier::NONE)),
                    b'D' => Some(Key::ArrowLeft(Modifier::NONE)),
                    b'P' => Some(Key::F(1, Modifier::NONE)),
                    b'Q' => Some(Key::F(2, Modifier::NONE)),
                    b'R' => Some(Key::F(3, Modifier::NONE)),
                    b'S' => Some(Key::F(4, Modifier::NONE)),
                    _ => None,
                };
                if let Some(k) = k {
                    out.push(k);
                    i += 3;
                    continue;
                }
            }
            // Alt-<key>（ESC + 次キー）
            if i + 1 < bytes.len() {
                let (k, used) = parse_one_key(&bytes[i + 1..]);
                out.push(add_alt(k));
                i += 1 + used;
                continue;
            } else {
                out.push(Key::Escape(Modifier::NONE));
                i += 1;
                continue;
            }
        }

        // 通常 1 キー
        let (k, used) = parse_one_key(&bytes[i..]);
        out.push(k);
        i += used;
    }

    out
}

fn parse_csi(bytes: &[u8]) -> (Option<Key>, usize) {
    if bytes.len() < 3 || bytes[0] != 0x1b || bytes[1] != b'[' {
        return (None, 0);
    }
    // [0-9;]* を吸う
    let mut j = 2;
    while j < bytes.len() && ((bytes[j] as char).is_ascii_digit() || bytes[j] == b';') {
        j += 1;
    }
    if j >= bytes.len() {
        return (None, 0);
    }
    let final_byte = bytes[j];

    let params_str = std::str::from_utf8(&bytes[2..j]).unwrap_or("");
    let mut it = params_str.split(';').filter(|s| !s.is_empty());
    let p1 = it.next().and_then(|s| s.parse::<u32>().ok());
    let p2 = it.next().and_then(|s| s.parse::<u32>().ok());

    // 矢印（A/B/C/D）: 修飾なし or 1;mod
    if matches!(final_byte, b'A' | b'B' | b'C' | b'D') {
        let m = if let Some(modcode) = p2 {
            mods_from_xterm_param(modcode)
        } else {
            Modifier::NONE
        };
        let key = match final_byte {
            b'A' => Key::ArrowUp(m),
            b'B' => Key::ArrowDown(m),
            b'C' => Key::ArrowRight(m),
            b'D' => Key::ArrowLeft(m),
            _ => unreachable!(),
        };
        return (Some(key), j + 1);
    }

    // Home / End（H/F）: 修飾なし or 1;mod
    if matches!(final_byte, b'H' | b'F') {
        let m = if let Some(modcode) = p2 {
            mods_from_xterm_param(modcode)
        } else {
            Modifier::NONE
        };
        let key = match final_byte {
            b'H' => Key::Home(m),
            b'F' => Key::End(m),
            _ => unreachable!(),
        };
        return (Some(key), j + 1);
    }

    // ~ 形式：Home/End/Insert/Delete/PageUp/PageDown + F1..F12
    if final_byte == b'~' {
        let m = if let Some(modcode) = p2 {
            mods_from_xterm_param(modcode)
        } else {
            Modifier::NONE
        };
        if let Some(n) = p1 {
            let key = match n {
                // Home / End
                1 | 7 => Key::Home(m),
                4 | 8 => Key::End(m),
                // Insert / Delete / PageUp / PageDown
                2 => Key::Insert(m),
                3 => Key::Delete(m),
                5 => Key::PageUp(m),
                6 => Key::PageDown(m),
                // F1..F12（11,12,13,14,15,17,18,19,20,21,23,24）
                11 => Key::F(1, m),
                12 => Key::F(2, m),
                13 => Key::F(3, m),
                14 => Key::F(4, m),
                15 => Key::F(5, m),
                17 => Key::F(6, m),
                18 => Key::F(7, m),
                19 => Key::F(8, m),
                20 => Key::F(9, m),
                21 => Key::F(10, m),
                23 => Key::F(11, m),
                24 => Key::F(12, m),
                _ => return (None, 0),
            };
            return (Some(key), j + 1);
        }
    }

    // 文字終端の F1..F4（P/Q/R/S）：ESC [ 1 ; <mod> <P|Q|R|S>
    if matches!(final_byte, b'P' | b'Q' | b'R' | b'S') {
        let m = if let Some(modcode) = p2 {
            mods_from_xterm_param(modcode)
        } else {
            Modifier::NONE
        };
        let n = match final_byte {
            b'P' => 1,
            b'Q' => 2,
            b'R' => 3,
            b'S' => 4,
            _ => unreachable!(),
        };
        return (Some(Key::F(n, m)), j + 1);
    }

    (None, 0)
}

/// ESC 以外先頭の 1 キーを読み取る
fn parse_one_key(bytes: &[u8]) -> (Key, usize) {
    let b = bytes[0];
    match b {
        0x0A | 0x0D => return (Key::Enter(Modifier::NONE), 1), // LF/CR
        0x09 => return (Key::Tab(Modifier::NONE), 1),          // HT
        0x7F => return (Key::Backspace(Modifier::NONE), 1),    // DEL
        0x08 => return (Key::Backspace(Modifier::NONE.with_ctrl()), 1), // BS = Ctrl-H
        0x00 => return (Key::Char('\0', Modifier::NONE.with_ctrl()), 1), // Ctrl-@/Ctrl-Space
        0x01..=0x1A => {
            // Ctrl-A..Z（Ctrl-I/TAB, Ctrl-M/CR は上で吸収済み）
            let ch = (b'a' + (b - 1)) as char;
            return (Key::Char(ch, Modifier::NONE.with_ctrl()), 1);
        }
        _ => {}
    }

    // ASCII printable（含むスペース）
    if b.is_ascii_graphic() || b == b' ' {
        return (Key::Char(b as char, Modifier::NONE), 1);
    }

    // UTF-8 多バイトを 1 文字だけ読む
    if b >= 0x80 {
        if let Ok(s) = std::str::from_utf8(bytes) {
            if let Some(ch) = s.chars().next() {
                return (Key::Char(ch, Modifier::NONE), ch.len_utf8());
            }
        }
        for len in 2..=4 {
            if bytes.len() >= len {
                if let Ok(s) = std::str::from_utf8(&bytes[..len]) {
                    if let Some(ch) = s.chars().next() {
                        return (Key::Char(ch, Modifier::NONE), ch.len_utf8());
                    }
                }
            }
        }
    }

    (Key::Unknown, 1)
}

fn add_alt(k: Key) -> Key {
    match k {
        Key::Char(c, m) => Key::Char(c, m.with_alt()),
        Key::Enter(m) => Key::Enter(m.with_alt()),
        Key::Backspace(m) => Key::Backspace(m.with_alt()),
        Key::Tab(m) => Key::Tab(m.with_alt()),
        Key::Escape(m) => Key::Escape(m.with_alt()),
        Key::ArrowUp(m) => Key::ArrowUp(m.with_alt()),
        Key::ArrowDown(m) => Key::ArrowDown(m.with_alt()),
        Key::ArrowLeft(m) => Key::ArrowLeft(m.with_alt()),
        Key::ArrowRight(m) => Key::ArrowRight(m.with_alt()),
        Key::Home(m) => Key::Home(m.with_alt()),
        Key::End(m) => Key::End(m.with_alt()),
        Key::PageUp(m) => Key::PageUp(m.with_alt()),
        Key::PageDown(m) => Key::PageDown(m.with_alt()),
        Key::Insert(m) => Key::Insert(m.with_alt()),
        Key::Delete(m) => Key::Delete(m.with_alt()),
        Key::F(n, m) => Key::F(n, m.with_alt()),
        Key::Unknown => Key::Unknown,
    }
}

/// xterm: 2=Shift, 3=Alt, 4=Shift+Alt, 5=Ctrl, 6=Shift+Ctrl, 7=Alt+Ctrl, 8=Shift+Alt+Ctrl
fn mods_from_xterm_param(p: u32) -> Modifier {
    let mut m = Modifier::NONE;
    if matches!(p, 2 | 4 | 6 | 8) {
        m = m.with_shift();
    }
    if matches!(p, 3 | 4 | 7 | 8) {
        m = m.with_alt();
    }
    #[allow(clippy::manual_range_patterns)]
    if matches!(p, 5 | 6 | 7 | 8) {
        m = m.with_ctrl();
    }
    m
}
