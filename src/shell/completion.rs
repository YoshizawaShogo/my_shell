/// 補完に必要な入力を整理。
/// cmd名、sub_cmd名、現在のポジション(cmd, sub_cmd, option, arg)、現在の入力情報
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, io,
};

use crate::error::Result;

#[derive(Default, Clone, Debug)]
pub struct CompletionStore {
    pub data: BTreeMap<String, CommandEntry>,
    pub path: String,
}

#[derive(Default, Clone, Debug)]
pub struct CommandEntry {
    pub options: BTreeSet<String>,
    pub subcommands: BTreeMap<String, SubcommandEntry>,
}

#[derive(Default, Clone, Debug)]
pub struct SubcommandEntry {
    pub options: BTreeSet<String>,
}

/// file format:
/// ## cmd
/// # subcmd
/// "--option" or "-o"
impl CompletionStore {
    pub(super) fn load() -> Result<Self> {
        let path = env::var("MY_SHELL_COMPLETION").unwrap_or_else(|_| {
            env::var("HOME").expect("HOME not set") + "/" + ".my_shell_completion"
        });
        let mut store = Self {
            data: BTreeMap::new(),
            path,
        };
        store.read_file()
    }
    pub(super) fn save(&self) -> Result<()> {
        self.write_file()
    }
    fn read_file(&mut self) -> Result<Self> {
        let content = match fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // ファイル未作成の場合は空でOK
                return Ok(self.clone());
            }
            Err(e) => return Err(e.into()),
        };

        let mut current_cmd: Option<String> = None;
        let mut current_sub: Option<String> = None;

        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with("//") {
                continue;
            } // コメント行を許容

            if let Some(cmd) = line.strip_prefix("## ") {
                let cmd = cmd.trim().to_string();
                self.data.entry(cmd.clone()).or_default();
                current_cmd = Some(cmd);
                current_sub = None;
                continue;
            }

            if let Some(sub) = line.strip_prefix("# ") {
                if let Some(cmd) = current_cmd.clone() {
                    let sub = sub.trim().to_string();
                    self.data
                        .entry(cmd)
                        .or_default()
                        .subcommands
                        .entry(sub.clone())
                        .or_default();
                    current_sub = Some(sub);
                }
                continue;
            }

            // オプション行（ダブルクォートで囲まれた語を全部拾う）
            if line.starts_with('"') {
                let opts = extract_quoted_items(line);
                if opts.is_empty() {
                    continue;
                }

                if let Some(cmd) = current_cmd.clone() {
                    if let Some(sub) = current_sub.clone() {
                        let subentry = self
                            .data
                            .entry(cmd)
                            .or_default()
                            .subcommands
                            .entry(sub)
                            .or_default();
                        for o in opts {
                            subentry.options.insert(o);
                        }
                    } else {
                        let centry = self.data.entry(cmd).or_default();
                        for o in opts {
                            centry.options.insert(o);
                        }
                    }
                }
            }
        }

        Ok(self.clone())
    }
    fn write_file(&self) -> Result<()> {
        let mut out = String::new();

        for (cmd, centry) in &self.data {
            out.push_str(&format!("## {}\n", cmd));

            // コマンド直下のオプション
            for opt in &centry.options {
                out.push_str(&format!("\"{}\"\n", opt));
            }
            if !centry.options.is_empty() {
                out.push('\n');
            }

            // サブコマンド
            for (sub, sentry) in &centry.subcommands {
                out.push_str(&format!("# {}\n", sub));
                for opt in &sentry.options {
                    out.push_str(&format!("\"{}\"\n", opt));
                }
                out.push('\n');
            }
        }

        fs::write(&self.path, out)?;
        Ok(())
    }
}

/// 行内の `"`で囲まれた部分文字列をすべて抽出する（例: `"--foo" or "-f"` → ["--foo", "-f"]）
fn extract_quoted_items(s: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut inq = false;
    let mut buf = String::new();

    for ch in s.chars() {
        if ch == '"' {
            if inq {
                // 終了
                items.push(buf.clone());
                buf.clear();
                inq = false;
            } else {
                // 開始
                inq = true;
            }
        } else if inq {
            buf.push(ch);
        }
    }
    items
}
