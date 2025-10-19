use std::collections::BTreeSet;
use std::process::{Command, Stdio};

use super::{Builtin, BuiltinResult};
use crate::shell::Shell;

pub struct CompleteCmd;

impl Builtin for CompleteCmd {
    fn name(&self) -> &'static str {
        "complete"
    }

    fn run(&self, shell: &mut Shell, argv: &[String]) -> BuiltinResult {
        // 引数はコマンド名のみ
        if argv.len() != 1 {
            return usage();
        }
        let cmd = argv[0].clone();
        let mut warnings = Vec::new();

        // ルート：man優先（抽出が空なら --help）
        let (root_opts, mut root_subs) = match fetch_best(&[cmd.as_str()], &mut warnings) {
            Some((opts, subs, _)) => (opts, subs),
            None => {
                return BuiltinResult {
                    stdout: String::new(),
                    stderr: format!("complete: no usable help/man content for `{}`\n", cmd),
                    code: 1,
                };
            }
        };

        // --- Git 特化のサブコマンド救済 ---
        // git の man/help は pager に流れたり、カテゴリ別の見出しで抽出しづらい場合がある。
        // 抽出ゼロ/少数のときは `git --list-cmds` / `git help -a` を使って補う。
        if cmd == "git" && root_subs.len() < 10 {
            match fetch_git_subcommands() {
                Ok(extra) if !extra.is_empty() => {
                    root_subs.extend(extra);
                }
                Ok(_) => {}
                Err(e) => {
                    warnings.push(format!("(warn) failed to enumerate git subcommands: {e}\n"))
                }
            }
        }

        // 保存（トップレベル）
        {
            let centry = shell.completion.data.entry(cmd.clone()).or_default();
            for o in &root_opts {
                centry.options.insert(o.clone());
            }
            for s in &root_subs {
                centry.subcommands.entry(s.clone()).or_default();
            }
        }

        // --- サブコマンドは1段のみ ---
        for sub in &root_subs {
            let segs = [cmd.as_str(), sub.as_str()];
            if let Some((sub_opts, _subsubs, _src)) = fetch_best(&segs, &mut warnings) {
                let centry = shell.completion.data.entry(cmd.clone()).or_default();
                let sentry = centry.subcommands.entry(sub.clone()).or_default();
                for o in &sub_opts {
                    sentry.options.insert(o.clone());
                }
            }
        }

        if let Err(e) = shell.completion.save() {
            warnings.push(format!("(warning) failed to save completion DB: {e}"));
        }

        BuiltinResult {
            stdout: format!("complete: updated `{}`\n", cmd),
            stderr: if warnings.is_empty() {
                String::new()
            } else {
                warnings.join("\n")
            },
            code: 0,
        }
    }
}

fn usage() -> BuiltinResult {
    BuiltinResult {
        stdout: String::new(),
        stderr: String::from(
            "Usage:\n  complete <command>\n\nCollect top-level options and one-level subcommands.\nPrefer man; fall back to --help only if parsing from man yields no results.\n",
        ),
        code: 1,
    }
}

// --------------------- Git 専用: サブコマンド列挙 ---------------------

fn fetch_git_subcommands() -> std::io::Result<BTreeSet<String>> {
    // 1) `git --list-cmds=main,others,alias,nohelpers` を試す（比較的新しめの Git）
    if let Ok(out) = Command::new("git")
        .arg("--list-cmds=main,others,alias,nohelpers")
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
        .env("LESS", "FRX")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        let txt = merge_out(&out.stdout, &out.stderr);
        let set = parse_git_list_cmds(&txt);
        if !set.is_empty() {
            return Ok(set);
        }
    }

    // 2) `git help -a` の出力を解析（古い Git 向け）
    if let Ok(out) = Command::new("git")
        .args(["help", "-a"]) // すべてのコマンド
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
        .env("LESS", "FRX")
        .env("TERM", "dumb")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        let txt = merge_out(&out.stdout, &out.stderr);
        let set = parse_git_help_a(&txt);
        if !set.is_empty() {
            return Ok(set);
        }
    }

    Ok(BTreeSet::new())
}

fn parse_git_list_cmds(text: &str) -> BTreeSet<String> {
    // 改行区切りでコマンド名が列挙される想定
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.split_whitespace().next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_git_help_a(text: &str) -> BTreeSet<String> {
    // `git help -a` はカテゴリー毎に一覧が表示される。
    // 行頭に2+スペースで始まるトークンを第一候補として拾う（説明文の先頭単語を誤検出しないようインデント基準）。
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let bytes = line.as_bytes();
        let leading_spaces = bytes.iter().take_while(|&&b| b == b' ').count();
        if leading_spaces >= 2 {
            if let Some(first) = line.trim_start().split_whitespace().next() {
                if first.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    set.insert(first.to_string());
                }
            }
        }
    }
    set
}

fn merge_out(stdout: &[u8], stderr: &[u8]) -> String {
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(stdout));
    if text.trim().is_empty() {
        text.push_str(&String::from_utf8_lossy(stderr));
    }
    text
}

// --------------------- 取得＆パース（共通） ---------------------

fn fetch_best(
    segments: &[&str],
    warnings: &mut Vec<String>,
) -> Option<(BTreeSet<String>, BTreeSet<String>, &'static str)> {
    // 1) man → 抽出
    match get_man_text(segments) {
        Ok(Some(text)) => {
            let o = extract_options_from_text(&text);
            let s = extract_subcommands(&text);
            if !o.is_empty() || !s.is_empty() {
                return Some((o, s, "man"));
            }
        }
        Ok(None) => {}
        Err(e) => warnings.push(format!(
            "(warn) man failed for `{}`: {e}",
            segments.join(" ")
        )),
    }

    // 2) help → 抽出
    match get_help_text(segments) {
        Ok(Some(text)) => {
            let o = extract_options_from_text(&text);
            let s = extract_subcommands(&text);
            if !o.is_empty() || !s.is_empty() {
                return Some((o, s, "help"));
            }
        }
        Ok(None) => {}
        Err(e) => warnings.push(format!(
            "(warn) --help failed for `{}`: {e}",
            segments.join(" ")
        )),
    }
    None
}

fn get_man_text(segments: &[&str]) -> std::io::Result<Option<String>> {
    // hyphen-joined (git-commit) → ルートcmd の順で試す
    let mut candidates = Vec::new();
    if segments.len() >= 2 {
        candidates.push(segments.join("-"));
    }
    candidates.push(segments[0].to_string());

    for page in candidates {
        let out = Command::new("man")
            .args(["-P", "cat", &page])
            .env("MANPAGER", "cat")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        match out {
            Ok(o) => {
                let txt = merge_out(&o.stdout, &o.stderr);
                if txt.trim().is_empty() {
                    continue;
                }
                let cleaned = strip_overstrike_and_controls(&txt);
                if cleaned.trim().is_empty() {
                    continue;
                }
                return Ok(Some(cleaned));
            }
            Err(_) => continue,
        }
    }
    Ok(None)
}

fn get_help_text(segments: &[&str]) -> std::io::Result<Option<String>> {
    if segments.is_empty() {
        return Ok(None);
    }
    let mut c = Command::new(segments[0]);
    for s in &segments[1..] {
        c.arg(s);
    }
    c.arg("--help")
        .env("PAGER", "cat")
        .env("GIT_PAGER", "cat")
        .env("LESS", "FRX")
        .env("TERM", "dumb");
    let out = c.stdout(Stdio::piped()).stderr(Stdio::piped()).output();
    match out {
        Ok(o) => {
            let txt = merge_out(&o.stdout, &o.stderr);
            if txt.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(txt))
            }
        }
        Err(e) => Err(e),
    }
}

fn strip_overstrike_and_controls(s: &str) -> String {
    // backspace overstrike + ANSI CSI の簡易除去
    let mut buf: Vec<char> = Vec::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\u{08}' => {
                buf.pop();
            }
            '\u{0C}' | '\u{00}' => {}
            _ => buf.push(ch),
        }
    }
    let mut out = String::with_capacity(buf.len());
    let mut i = 0;
    while i < buf.len() {
        let c = buf[i];
        if c == '\u{1b}' {
            i += 1;
            if i < buf.len() && buf[i] == '[' {
                i += 1;
                while i < buf.len() {
                    let cc = buf[i];
                    if cc.is_ascii_alphabetic() {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

// --------------------- 抽出（共通） ---------------------

pub fn extract_options_from_text(text: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for raw in text.split_whitespace() {
        let token =
            raw.trim_matches(|c: char| matches!(c, ',' | ';' | ':' | ')' | '(' | '[' | ']'));
        if let Some(rest) = token.strip_prefix("--") {
            if !rest
                .chars()
                .next()
                .map(|c| c.is_ascii_alphanumeric())
                .unwrap_or(false)
            {
                continue;
            }
            let name = rest.split(&['=', ' '][..]).next().unwrap_or(rest);
            let clean = name
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect::<String>();
            if !clean.is_empty() {
                set.insert(format!("--{}", clean));
            }
            continue;
        }
        if let Some(rest) = token.strip_prefix('-') {
            if rest.is_empty() || rest.starts_with('-') {
                continue;
            }
            for ch in rest.chars() {
                if ch.is_ascii_alphabetic() {
                    set.insert(format!("-{}", ch));
                } else {
                    break;
                }
            }
        }
    }
    set
}

pub fn extract_subcommands(text: &str) -> BTreeSet<String> {
    let mut subs = BTreeSet::new();
    let lines: Vec<&str> = text.lines().collect();

    let headings = [
        "SUBCOMMANDS:",
        "Subcommands:",
        "Sub-Commands:",
        "COMMANDS:",
        "Commands:",
        "Available commands:",
        "GIT COMMANDS",
        "Git Commands",
        "git commands",
    ];

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if headings.iter().any(|h| line.starts_with(h)) {
            i += 1;
            while i < lines.len() {
                let l = lines[i];
                if l.trim().is_empty() {
                    break;
                }
                // 次のセクション見出しらしき行で終了
                if (l.trim_end().ends_with(":") && l == l.trim())
                    || l.chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_whitespace())
                {
                    break;
                }
                let token = l.trim_start().split_whitespace().next().unwrap_or("");
                let token = token.trim_matches(|c: char| matches!(c, ',' | ';'));
                let clean: String = token
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                if !clean.is_empty() {
                    subs.insert(clean);
                }
                i += 1;
            }
            continue;
        }
        i += 1;
    }

    subs
}
