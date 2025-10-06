use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::shell::pipeline::parse::{CommandExpr, Expr, WordNode};

#[derive(Default, Clone, Debug)]
pub struct CompletionStore {
    data: BTreeMap<String, CommandEntry>,
    path: PathBuf,
}

#[derive(Default, Clone, Debug)]
pub struct CommandEntry {
    pub options: BTreeSet<String>,
    pub subcommands: BTreeMap<String, SubcommandEntry>,
    pub filter: CompletionFilter,
}

#[derive(Default, Clone, Debug)]
pub struct SubcommandEntry {
    pub options: BTreeSet<String>,
    pub filter: CompletionFilter,
}

#[derive(Clone, Debug, Default)]
pub struct CompletionFilter {
    pub type_filter: Option<char>,
    pub require_exec: bool,
}

impl CompletionStore {
    pub fn load(path: PathBuf) -> Self {
        let mut store = Self {
            data: BTreeMap::new(),
            path,
        };
        store.read_file();
        store
    }

    fn read_file(&mut self) {
        let Ok(content) = fs::read_to_string(&self.path) else {
            return;
        };

        let mut current_command: Option<String> = None;
        let mut current_sub: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix('#') {
                if let Some(cmd_name) = current_command.clone() {
                    let update = parse_filter_comment(rest);
                    if let Some(sub_name) = current_sub.clone() {
                        let entry = self.data.entry(cmd_name).or_default();
                        entry
                            .subcommands
                            .entry(sub_name)
                            .or_default()
                            .filter
                            .merge(&update);
                    } else {
                        let entry = self.data.entry(cmd_name).or_default();
                        entry.filter.merge(&update);
                    }
                }
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("%%") {
                let name = rest.trim();
                if name.is_empty() {
                    current_sub = None;
                    continue;
                }
                if let Some(cmd) = current_command.clone() {
                    let entry = self.data.entry(cmd.clone()).or_default();
                    entry.subcommands.entry(name.to_string()).or_default();
                    current_sub = Some(name.to_string());
                }
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix('%') {
                let name = rest.trim();
                if name.is_empty() {
                    current_command = None;
                    current_sub = None;
                    continue;
                }
                let name = name.to_string();
                self.data.entry(name.clone()).or_default();
                current_command = Some(name);
                current_sub = None;
                continue;
            }
            if let Some(option) = parse_option_line(trimmed) {
                if let Some(cmd) = current_command.clone() {
                    let entry = self.data.entry(cmd).or_default();
                    if let Some(sub) = current_sub.clone() {
                        entry
                            .subcommands
                            .entry(sub)
                            .or_default()
                            .options
                            .insert(option);
                    } else {
                        entry.options.insert(option);
                    }
                }
            }
        }
    }

    pub fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut file = File::create(&self.path)?;
        for (cmd, entry) in &self.data {
            writeln!(file, "%{}", cmd)?;
            if let Some(line) = format_filter(&entry.filter) {
                writeln!(file, "# {}", line)?;
            }
            for opt in &entry.options {
                writeln!(file, "{}", escape_option(opt))?;
            }
            for (sub, sub_entry) in &entry.subcommands {
                writeln!(file, "%% {}", sub)?;
                if let Some(line) = format_filter(&sub_entry.filter) {
                    writeln!(file, "# {}", line)?;
                }
                for opt in &sub_entry.options {
                    writeln!(file, "{}", escape_option(opt))?;
                }
            }
        }
        Ok(())
    }

    pub fn record_tokens(&mut self, tokens: &[String]) {
        if tokens.is_empty() {
            return;
        }
        let command_raw = &tokens[0];
        let command = command_name(command_raw);
        let entry = self.data.entry(command.clone()).or_default();

        let mut iter = tokens.iter().skip(1);
        let maybe_first = iter.next();
        let mut subcommand: Option<String> = None;
        if let Some(first) = maybe_first {
            if !first.starts_with('-') && !looks_like_path(first) {
                subcommand = Some(first.clone());
                entry.subcommands.entry(first.clone()).or_default();
            }
        }

        for token in tokens.iter().skip(1) {
            if let Some(opt) = canonicalize_option(token) {
                if let Some(sub) = subcommand.clone() {
                    entry
                        .subcommands
                        .entry(sub.clone())
                        .or_default()
                        .options
                        .insert(opt);
                } else {
                    entry.options.insert(opt);
                }
            }
        }
    }

    pub fn record_help_output(&mut self, tokens: &[String], help_output: &str) {
        if tokens.is_empty() {
            return;
        }
        let options = parse_options_from_help(help_output);
        if options.is_empty() {
            return;
        }
        let command = command_name(&tokens[0]);
        let entry = self.data.entry(command).or_default();

        let mut subcommand: Option<String> = None;
        if let Some(first) = tokens.get(1) {
            if !first.starts_with('-') && !looks_like_path(first) {
                subcommand = Some(first.clone());
                entry.subcommands.entry(first.clone()).or_default();
            }
        }

        if let Some(sub) = subcommand {
            if let Some(sub_entry) = entry.subcommands.get_mut(&sub) {
                for opt in options {
                    if let Some(canonical) = canonicalize_option(&opt) {
                        sub_entry.options.insert(canonical);
                    }
                }
            }
        } else {
            for opt in options {
                if let Some(canonical) = canonicalize_option(&opt) {
                    entry.options.insert(canonical);
                }
            }
        }
    }

    pub fn commands(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    pub fn subcommands(&self, command: &str) -> Option<impl Iterator<Item = &String>> {
        self.data.get(command).map(|entry| entry.subcommands.keys())
    }

    pub fn subcommand_options(
        &self,
        command: &str,
        subcommand: &str,
    ) -> Option<impl Iterator<Item = &String>> {
        self.data
            .get(command)
            .and_then(|entry| entry.subcommands.get(subcommand))
            .map(|sub| sub.options.iter())
    }

    pub fn command_options(&self, command: &str) -> Option<impl Iterator<Item = &String>> {
        self.data.get(command).map(|entry| entry.options.iter())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn command_filter(&self, command: &str) -> CompletionFilter {
        self.data
            .get(command)
            .map(|entry| entry.filter.clone())
            .unwrap_or_default()
    }

    pub fn subcommand_filter(&self, command: &str, subcommand: &str) -> CompletionFilter {
        self.data
            .get(command)
            .and_then(|entry| entry.subcommands.get(subcommand))
            .map(|sub| sub.filter.clone())
            .unwrap_or_default()
    }

    pub fn ensure_command_filter(&mut self, command: &str, default_filter: CompletionFilter) {
        let entry = self.data.entry(command.to_string()).or_default();
        entry.filter.apply_defaults(&default_filter);
    }
}

pub(crate) fn command_name(raw: &str) -> String {
    if raw.contains('/') {
        Path::new(raw)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(raw)
            .to_string()
    } else {
        raw.to_string()
    }
}

pub(crate) fn looks_like_path(value: &str) -> bool {
    if value.contains('/') {
        return true;
    }
    if matches!(value, "." | "..") {
        return true;
    }
    Path::new(value).exists()
}

fn parse_option_line(line: &str) -> Option<String> {
    let raw = if let Some(stripped) = line.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        unescape_option(stripped)?
    } else if line.trim_start().starts_with('-') {
        line.trim().to_string()
    } else {
        return None;
    };
    canonicalize_option(&raw)
}

fn escape_option(opt: &str) -> String {
    let mut escaped = String::with_capacity(opt.len() + 2);
    escaped.push('"');
    for ch in opt.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            _ => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

fn unescape_option(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let next = chars.next()?;
            out.push(next);
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

fn canonicalize_option(option: &str) -> Option<String> {
    let mut opt = option.trim();
    if opt.is_empty() {
        return None;
    }

    while matches!(opt.chars().next(), Some('[' | '(' | '{'))
        && matches!(opt.chars().last(), Some(']' | ')' | '}'))
    {
        opt = opt
            .trim_start_matches(|c| matches!(c, '[' | '(' | '{'))
            .trim_end_matches(|c| matches!(c, ']' | ')' | '}'))
            .trim();
    }

    if !opt.starts_with('-') {
        return None;
    }

    let mut opt = opt
        .trim_end_matches(|c: char| matches!(c, ',' | ';' | ':' | '.'))
        .trim()
        .to_string();

    while matches!(opt.chars().last(), Some(']' | ')' | '}')) {
        opt.pop();
        opt = opt.trim_end().to_string();
    }

    if opt.is_empty() || !opt.starts_with('-') {
        return None;
    }

    if let Some(eq_pos) = opt.find('=') {
        let base = opt[..eq_pos]
            .trim_end_matches(|c: char| matches!(c, '[' | '(' | '{'))
            .to_string();
        if base.is_empty() || !base.starts_with('-') {
            return None;
        }
        let mut canonical = base;
        canonical.push('=');
        return Some(canonical);
    }

    let trimmed = opt
        .trim_end_matches(|c: char| matches!(c, '[' | '(' | '{'))
        .to_string();
    if trimmed.is_empty() || !trimmed.starts_with('-') {
        return None;
    }

    let mut canonical = trimmed;
    if !canonical.ends_with(' ') {
        canonical.push(' ');
    }
    Some(canonical)
}

impl CompletionFilter {
    pub fn merge(&mut self, other: &CompletionFilter) {
        if let Some(t) = other.type_filter {
            self.type_filter = Some(t);
        }
        if other.require_exec {
            self.require_exec = true;
        }
    }

    fn apply_defaults(&mut self, default: &CompletionFilter) {
        if self.type_filter.is_none() {
            self.type_filter = default.type_filter;
        }
        if default.require_exec {
            self.require_exec = true;
        }
    }

    fn is_default(&self) -> bool {
        self.type_filter.is_none() && !self.require_exec
    }
}

fn parse_filter_comment(comment: &str) -> CompletionFilter {
    let mut filter = CompletionFilter::default();
    for part in comment.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(value) = part.strip_prefix("type=") {
            let value = value.trim();
            if let Some(ch) = value.chars().next() {
                filter.type_filter = Some(ch);
            }
        } else if let Some(value) = part.strip_prefix("perm=") {
            if value.chars().any(|c| c == 'x' || c == 'X') {
                filter.require_exec = true;
            }
        }
    }
    filter
}

fn format_filter(filter: &CompletionFilter) -> Option<String> {
    if filter.is_default() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some(t) = filter.type_filter {
        parts.push(format!("type={}", t));
    }
    if filter.require_exec {
        parts.push("perm=x".to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn parse_options_from_help(help_output: &str) -> Vec<String> {
    let mut options = BTreeSet::new();
    for line in help_output.lines() {
        for token in line.split_whitespace() {
            let cleaned = token.trim_matches(|c: char| matches!(c, ',' | ';'));
            if let Some(opt) = canonicalize_option(cleaned) {
                options.insert(opt);
            }
        }
    }
    options.into_iter().collect()
}

pub fn default_completion_path() -> PathBuf {
    env::var("MY_SHELL_COMPLETION")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".my_shell_completion"))
        })
        .unwrap_or_else(|| PathBuf::from(".my_shell_completion"))
}

pub fn commands_from_expr(expr: &Expr) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    collect_commands(expr, &mut out);
    out
}

fn collect_commands(expr: &Expr, out: &mut Vec<Vec<String>>) {
    match expr {
        Expr::And(lhs, rhs) => {
            collect_commands(lhs, out);
            collect_commands(rhs, out);
        }
        Expr::Or(lhs, rhs) => {
            collect_commands(lhs, out);
            collect_commands(rhs, out);
        }
        Expr::Pipe(cmds) => {
            for cmd in cmds {
                out.push(command_to_tokens(cmd));
            }
        }
    }
}

fn command_to_tokens(cmd: &CommandExpr) -> Vec<String> {
    let mut tokens = Vec::new();
    tokens.push(word_to_string(&cmd.cmd_name));
    for arg in &cmd.args {
        tokens.push(word_to_string(arg));
    }
    tokens
}

fn word_to_string(word: &WordNode) -> String {
    word.concat_text()
}
