use my_shell::term_mode;
use std::io::{self, Read, Write, stderr, stdin, stdout};

fn main() {
    // 現在のterminal情報を取得

    let mut stdout = io::stdout().lock();
    while let Some(b) = stdin().lock().bytes().next() {
        let b = match b {
            Ok(b) => b,
            Err(e) => {
                println!("{:?}", e);
                unreachable!();
            }
        };

        match b {
            4 => return,
            _ => {
                if b.is_ascii_graphic() || b == b' ' {
                    writeln!(
                        stdout,
                        "文字: '{}', 10進: {}, 16進: 0x{:X}",
                        b as char, b, b
                    )
                    .unwrap();
                } else {
                    writeln!(
                        stdout,
                        "制御: ^{}, 10進: {}, 16進: 0x{:X}",
                        (b + 0x40) as char,
                        b,
                        b
                    )
                    .unwrap();
                }
                stdout.flush().unwrap();
            }
        }
    }
}
