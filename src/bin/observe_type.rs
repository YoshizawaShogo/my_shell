use std::io::{self, Read};

fn main() {
    println!("文字を入力してください (Ctrl+Dで終了)");

    let stdin = io::stdin();
    for byte in stdin.lock().bytes() {
        match byte {
            Ok(b) => {
                let ch = b as char;
                println!("文字: '{}', 10進数: {}, 16進数: 0x{:X}", ch, b, b);
            }
            Err(_) => {
                println!("\n入力エラーが発生しました。");
                break;
            }
        }
    }

    println!("終了します。");
}
