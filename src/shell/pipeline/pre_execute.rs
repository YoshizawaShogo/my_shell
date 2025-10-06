// pre_execute.rs
use std::{
    collections::{BTreeMap, HashMap},
    env,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::shell::{
    Shell,
    pipeline::parse::{CommandExpr, Expr, Redirection, Segment, WordNode},
};

/// シェル変数 + 環境変数のスコープ
struct VarScope {
    vars: HashMap<String, String>, // Shell.variables のスナップショット
}

impl VarScope {
    fn from_btree(b: &BTreeMap<String, String>) -> Self {
        Self {
            vars: b.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        }
    }
    #[inline]
    fn get(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned().or_else(|| env::var(name).ok())
    }
}

/// $VAR / ${VAR} の展開（SingleQuoted 以外で使う）
fn expand_str(s: &str, scope: &VarScope) -> String {
    #[inline]
    fn is_start(c: char) -> bool {
        c == '_' || c.is_ascii_alphabetic()
    }
    #[inline]
    fn is_tail(c: char) -> bool {
        c == '_' || c.is_ascii_alphanumeric()
    }

    use std::iter::Peekable;
    use std::str::Chars;

    let mut out = String::with_capacity(s.len());
    let mut it: Peekable<Chars<'_>> = s.chars().peekable();

    while let Some(c) = it.next() {
        match c {
            '\\' => {
                if let Some(nc) = it.peek().copied() {
                    if nc == '$' {
                        it.next(); // consume '$'
                        out.push('$');
                    } else {
                        out.push('\\');
                        out.push(nc);
                        it.next();
                    }
                } else {
                    out.push('\\');
                }
            }
            '$' => match it.peek().copied() {
                Some('{') => {
                    it.next(); // consume '{'
                    let mut name = String::new();
                    while let Some(ch) = it.next() {
                        if ch == '}' {
                            break;
                        }
                        name.push(ch);
                    }
                    let val = scope.get(&name).unwrap_or_default();
                    out.push_str(&val);
                }
                Some(p) if is_start(p) => {
                    let mut name = String::new();
                    name.push(p);
                    it.next(); // first
                    while let Some(&t) = it.peek() {
                        if is_tail(t) {
                            name.push(t);
                            it.next();
                        } else {
                            break;
                        }
                    }
                    let val = scope.get(&name).unwrap_or_default();
                    out.push_str(&val);
                }
                _ => out.push('$'),
            },
            _ => out.push(c),
        }
    }
    out
}

/// WordNode 内の各セグメントに展開適用（SingleQuoted は素通し）
fn expand_wordnode(node: &WordNode, scope: &VarScope) -> WordNode {
    let mut out = WordNode {
        segments: Vec::with_capacity(node.segments.len()),
    };
    for seg in &node.segments {
        match seg {
            Segment::SingleQuoted(t) => out.segments.push(Segment::SingleQuoted(t.clone())),
            Segment::Unquoted(t) => out.segments.push(Segment::Unquoted(expand_str(t, scope))),
            Segment::DoubleQuoted(t) => out
                .segments
                .push(Segment::DoubleQuoted(expand_str(t, scope))),
        }
    }
    out
}

fn expand_redirection(r: &Redirection, scope: &VarScope) -> Redirection {
    match r {
        Redirection::File { path, append } => Redirection::File {
            path: expand_wordnode(path, scope),
            append: *append,
        },
        Redirection::Pipe => Redirection::Pipe,
        Redirection::Inherit => Redirection::Inherit,
    }
}

fn expand_command(cmd: &CommandExpr, scope: &VarScope) -> CommandExpr {
    let expanded_cmd = CommandExpr {
        cmd_name: expand_wordnode(&cmd.cmd_name, scope),
        args: cmd.args.iter().map(|w| expand_wordnode(w, scope)).collect(),
        stdout: expand_redirection(&cmd.stdout, scope),
        stderr: expand_redirection(&cmd.stderr, scope),
    };
    expand_globs(expanded_cmd)
}

/// 公開API: Shell（Arc<Mutex<…>>）から variables をスナップショットして展開
pub fn expand_expr_with_shell(expr: &Expr, shell: &Arc<Mutex<Shell>>) -> Expr {
    // できるだけ短時間でロックを解放するため、スナップショットを作る
    let snapshot: BTreeMap<String, String> = if let Ok(guard) = shell.lock() {
        guard.variables.clone()
    } else {
        BTreeMap::new() // ロックが取れなければ空で続行（好みで挙動変更可）
    };
    let scope = VarScope::from_btree(&snapshot);
    expand_expr_with_scope(expr, &scope)
}

/// テストやカスタムスコープ向け（スナップショットを外から渡す）
fn expand_expr_with_scope(expr: &Expr, scope: &VarScope) -> Expr {
    match expr {
        Expr::And(a, b) => Expr::And(
            Box::new(expand_expr_with_scope(a, scope)),
            Box::new(expand_expr_with_scope(b, scope)),
        ),
        Expr::Or(a, b) => Expr::Or(
            Box::new(expand_expr_with_scope(a, scope)),
            Box::new(expand_expr_with_scope(b, scope)),
        ),
        Expr::Pipe(cmds) => {
            let new_cmds = cmds.iter().map(|c| expand_command(c, scope)).collect();
            Expr::Pipe(new_cmds)
        }
    }
}

fn expand_globs(mut cmd: CommandExpr) -> CommandExpr {
    cmd.args = cmd
        .args
        .into_iter()
        .flat_map(|arg| expand_word_glob(&arg).unwrap_or_else(|| vec![arg]))
        .collect();
    cmd
}

fn expand_word_glob(word: &WordNode) -> Option<Vec<WordNode>> {
    let mut combined = String::new();
    for segment in &word.segments {
        match segment {
            Segment::Unquoted(text) => combined.push_str(text),
            _ => return None,
        }
    }
    if !combined.contains('*') {
        return None;
    }
    let matches = expand_glob_pattern(&combined);
    if matches.is_empty() {
        return None;
    }
    Some(
        matches
            .into_iter()
            .map(|m| WordNode {
                segments: vec![Segment::Unquoted(m)],
            })
            .collect(),
    )
}

fn expand_glob_pattern(pattern: &str) -> Vec<String> {
    let is_absolute = Path::new(pattern).is_absolute();
    let mut components: Vec<&str> = pattern.split('/').collect();
    if is_absolute && components.first() == Some(&"") {
        components.remove(0);
    }

    let mut paths = if is_absolute {
        vec![PathBuf::from("/")]
    } else {
        vec![PathBuf::from(".")]
    };

    for comp in components {
        if comp.is_empty() || comp == "." {
            continue;
        }
        let has_wildcard = comp.contains('*');
        let mut next_paths = Vec::new();

        if has_wildcard {
            for base in &paths {
                if let Ok(entries) = fs::read_dir(base) {
                    for entry in entries.flatten() {
                        let name_os = entry.file_name();
                        let name = match name_os.to_str() {
                            Some(s) => s,
                            None => continue,
                        };
                        if name.starts_with('.') && !comp.starts_with('.') {
                            continue;
                        }
                        if wildcard_match(comp, name) {
                            let mut next = base.clone();
                            next.push(name);
                            next_paths.push(next);
                        }
                    }
                }
            }
        } else {
            for base in &paths {
                let mut next = base.clone();
                next.push(comp);
                if next.exists() {
                    next_paths.push(next);
                }
            }
        }

        paths = next_paths;
        if paths.is_empty() {
            break;
        }
    }

    if paths.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();
    for path in paths {
        let mut display = if is_absolute {
            path.to_string_lossy().to_string()
        } else if let Ok(stripped) = path.strip_prefix(".") {
            let s = stripped.to_string_lossy().to_string();
            if s.is_empty() {
                String::from('.')
            } else {
                s
            }
        } else {
            path.to_string_lossy().to_string()
        };
        if !is_absolute && display.starts_with("./") {
            display = display[2..].to_string();
        }
        results.push(display);
    }
    results.sort();
    results.dedup();
    results
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p_bytes = pattern.as_bytes();
    let t_bytes = text.as_bytes();
    let mut p = 0;
    let mut t = 0;
    let mut star_idx: Option<usize> = None;
    let mut match_idx = 0;

    while t < t_bytes.len() {
        if p < p_bytes.len() && (p_bytes[p] == t_bytes[t]) {
            p += 1;
            t += 1;
        } else if p < p_bytes.len() && p_bytes[p] == b'*' {
            star_idx = Some(p);
            match_idx = t;
            p += 1;
        } else if let Some(star_pos) = star_idx {
            p = star_pos + 1;
            match_idx += 1;
            t = match_idx;
        } else {
            return false;
        }
    }

    while p < p_bytes.len() && p_bytes[p] == b'*' {
        p += 1;
    }

    p == p_bytes.len()
}

fn expand_glob_pattern(pattern: &str) gives Vec<String>? We'll add soon.
