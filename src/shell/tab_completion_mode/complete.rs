use std::{
    env::current_dir,
    fs::read_dir,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::shell::{
    Shell,
    completion::{command_name, looks_like_path},
};

#[derive(Debug, Clone)]
struct Candidate {
    value: String,
    plain_display: String,
    styled_display: String,
    append_slash: bool,
}

#[derive(Debug, Clone)]
struct FileCandidate {
    name: String,
    is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct DisplayEntry {
    pub plain: String,
    pub styled: String,
}

enum Context {
    File,
    Command,
    PotentialSubcommand {
        command: String,
    },
    Option {
        command: String,
        subcommand: Option<String>,
    },
}

type CandidateList = Vec<Candidate>;

pub fn complete(
    shell: &Arc<Mutex<Shell>>,
    buffer: &mut String,
    cursor: &mut usize,
) -> Option<Vec<DisplayEntry>> {
    let original_buffer = buffer.clone();
    let original_cursor = *cursor;

    let (token_start, token_slice) = extract_token(buffer, *cursor);
    let token = token_slice.to_string();
    let (dir_part, file_prefix) = split_token(&token);

    let segment_start = find_segment_start(&buffer[..*cursor]);
    let segment = &buffer[segment_start..*cursor];
    let words: Vec<String> = segment.split_whitespace().map(|s| s.to_string()).collect();
    let token_is_empty = token.is_empty();
    let current_index = if token_is_empty {
        words.len()
    } else {
        words.len().saturating_sub(1)
    };

    let command_opt = words.get(0).map(|w| command_name(w));
    let subcommand_opt = words.get(1).and_then(|w| {
        if w.starts_with('-') || looks_like_path(w) {
            None
        } else {
            Some(w.clone())
        }
    });

    let context = determine_context(
        &dir_part,
        &token,
        current_index,
        command_opt.clone(),
        subcommand_opt.clone(),
    );

    let (mut candidates, prefix, file_mode) = match context {
        Context::File => {
            let candidates = collect_file_candidates(&dir_part, &file_prefix);
            let transformed = candidates
                .into_iter()
                .map(|c| {
                    let plain = plain_file_display(&c);
                    Candidate {
                        value: c.name.clone(),
                        plain_display: plain.clone(),
                        styled_display: styled_display(&plain, &file_prefix),
                        append_slash: c.is_dir,
                    }
                })
                .collect::<CandidateList>();
            (transformed, file_prefix.clone(), true)
        }
        Context::Command => {
            let prefix = token.clone();
            let candidates = gather_command_candidates(shell, &prefix);
            let transformed = candidates
                .into_iter()
                .map(|value| {
                    let plain = value.clone();
                    Candidate {
                        styled_display: styled_display(&plain, &prefix),
                        plain_display: plain,
                        value,
                        append_slash: false,
                    }
                })
                .collect::<CandidateList>();
            (transformed, prefix, false)
        }
        Context::PotentialSubcommand { command } => {
            let prefix = token.clone();
            let mut candidates = gather_subcommand_candidates(shell, &command, &prefix);
            if candidates.is_empty() {
                let file_candidates = collect_file_candidates(&dir_part, &file_prefix);
                let transformed = file_candidates
                    .into_iter()
                    .map(|c| {
                        let plain = plain_file_display(&c);
                        Candidate {
                            value: c.name.clone(),
                            plain_display: plain.clone(),
                            styled_display: styled_display(&plain, &file_prefix),
                            append_slash: c.is_dir,
                        }
                    })
                    .collect::<CandidateList>();
                (transformed, file_prefix.clone(), true)
            } else {
                candidates.sort();
                let transformed = candidates
                    .into_iter()
                    .map(|value| {
                        let plain = value.clone();
                        Candidate {
                            styled_display: styled_display(&plain, &prefix),
                            plain_display: plain,
                            value,
                            append_slash: false,
                        }
                    })
                    .collect::<CandidateList>();
                (transformed, prefix, false)
            }
        }
        Context::Option {
            command,
            subcommand,
        } => {
            let prefix = token.clone();
            let candidates =
                gather_option_candidates(shell, &command, subcommand.as_deref(), &prefix);
            let transformed = candidates
                .into_iter()
                .map(|value| {
                    let plain = value.clone();
                    Candidate {
                        styled_display: styled_display(&plain, &prefix),
                        plain_display: plain,
                        value,
                        append_slash: false,
                    }
                })
                .collect::<CandidateList>();
            (transformed, prefix, false)
        }
    };

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|a, b| a.value.cmp(&b.value));
    let names: Vec<&str> = candidates.iter().map(|c| c.value.as_str()).collect();

    if candidates.len() == 1 {
        let candidate = &candidates[0];
        apply_candidate(buffer, cursor, token_start, &dir_part, candidate, file_mode);
        return None;
    }

    let common_prefix = longest_common_prefix(&names);
    if common_prefix.len() > prefix.len() {
        if let Some(candidate) = candidates.iter().find(|c| c.value == common_prefix) {
            apply_candidate(buffer, cursor, token_start, &dir_part, candidate, file_mode);
        } else {
            let mut replacement = dir_part.clone();
            replacement.push_str(&common_prefix);
            buffer.replace_range(token_start..*cursor, &replacement);
            *cursor = token_start + replacement.len();
        }
        return None;
    }

    let mut display = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        display.push(DisplayEntry {
            plain: candidate.plain_display,
            styled: candidate.styled_display,
        });
    }

    *buffer = original_buffer;
    *cursor = original_cursor;

    Some(display)
}

fn apply_candidate(
    buffer: &mut String,
    cursor: &mut usize,
    token_start: usize,
    dir_part: &str,
    candidate: &Candidate,
    file_mode: bool,
) {
    let mut replacement = if file_mode {
        let mut rep = dir_part.to_string();
        rep.push_str(&candidate.value);
        if candidate.append_slash && !rep.ends_with('/') {
            rep.push('/');
        }
        rep
    } else {
        candidate.value.clone()
    };
    if !file_mode && candidate.append_slash {
        replacement.push('/');
    }
    buffer.replace_range(token_start..*cursor, &replacement);
    *cursor = token_start + replacement.len();
}

fn determine_context(
    dir_part: &str,
    token: &str,
    current_index: usize,
    command_opt: Option<String>,
    subcommand_opt: Option<String>,
) -> Context {
    if !dir_part.is_empty() {
        return Context::File;
    }
    if command_opt.is_none() {
        return Context::Command;
    }
    let command = command_opt.unwrap();
    if current_index == 0 {
        return Context::Command;
    }
    if token.starts_with('-') {
        return Context::Option {
            command,
            subcommand: subcommand_opt,
        };
    }
    if current_index == 1 {
        return Context::PotentialSubcommand { command };
    }
    if token.starts_with('-') {
        return Context::Option {
            command,
            subcommand: subcommand_opt,
        };
    }
    Context::File
}

fn gather_command_candidates(shell: &Arc<Mutex<Shell>>, prefix: &str) -> Vec<String> {
    let mut exe_candidates = Vec::new();
    let mut store_candidates = Vec::new();
    if let Ok(mut sh) = shell.lock() {
        exe_candidates = sh.exe_list.command_candidates(prefix);
        store_candidates = sh
            .completion
            .commands()
            .filter(|name| name.starts_with(prefix))
            .cloned()
            .collect();
    }
    let mut set = std::collections::BTreeSet::new();
    set.extend(exe_candidates);
    set.extend(store_candidates);
    set.into_iter().collect()
}

fn gather_subcommand_candidates(
    shell: &Arc<Mutex<Shell>>,
    command: &str,
    prefix: &str,
) -> Vec<String> {
    if let Ok(sh) = shell.lock() {
        if let Some(iter) = sh.completion.subcommands(command) {
            return iter
                .filter(|sub| sub.starts_with(prefix))
                .cloned()
                .collect();
        }
    }
    Vec::new()
}

fn gather_option_candidates(
    shell: &Arc<Mutex<Shell>>,
    command: &str,
    subcommand: Option<&str>,
    prefix: &str,
) -> Vec<String> {
    let mut opts = std::collections::BTreeSet::new();
    if let Ok(sh) = shell.lock() {
        if let Some(iter) = sh.completion.command_options(command) {
            for opt in iter {
                if opt.starts_with(prefix) {
                    opts.insert(opt.clone());
                }
            }
        }
        if let Some(sub) = subcommand {
            if let Some(iter) = sh.completion.subcommand_options(command, sub) {
                for opt in iter {
                    if opt.starts_with(prefix) {
                        opts.insert(opt.clone());
                    }
                }
            }
        }
    }
    opts.into_iter().collect()
}

fn plain_file_display(candidate: &FileCandidate) -> String {
    let mut name = candidate.name.clone();
    if candidate.is_dir {
        name.push('/');
    }
    name
}

fn styled_display(plain: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        if plain.is_empty() {
            String::new()
        } else {
            format!("\x1b[90m{}\x1b[0m", plain)
        }
    } else if let Some(rest) = plain.strip_prefix(prefix) {
        if rest.is_empty() {
            plain.to_string()
        } else {
            format!("{}\x1b[90m{}\x1b[0m", prefix, rest)
        }
    } else {
        plain.to_string()
    }
}

fn extract_token(buffer: &str, cursor: usize) -> (usize, &str) {
    let upto_cursor = &buffer[..cursor];
    let start = upto_cursor
        .rfind(|c: char| c.is_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(0);
    (start, &buffer[start..cursor])
}

fn split_token(token: &str) -> (String, String) {
    match token.rfind('/') {
        Some(idx) => {
            let (dir, file) = token.split_at(idx + 1);
            (dir.to_string(), file.to_string())
        }
        None => (String::new(), token.to_string()),
    }
}

fn find_segment_start(input: &str) -> usize {
    let mut start = 0;
    for (idx, ch) in input.char_indices() {
        if matches!(ch, '|' | '&' | ';') {
            start = idx + 1;
        }
    }
    start
}

fn collect_file_candidates(dir_part: &str, file_prefix: &str) -> Vec<FileCandidate> {
    let search_dir = match resolve_search_dir(dir_part) {
        Some(dir) => dir,
        None => return Vec::new(),
    };
    let Ok(entries) = read_dir(search_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name();
            let name = file_name.to_str()?.to_string();
            if !file_prefix.starts_with('.') && name.starts_with('.') {
                return None;
            }
            if !name.starts_with(file_prefix) {
                return None;
            }
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            Some(FileCandidate { name, is_dir })
        })
        .collect()
}

fn resolve_search_dir(dir_part: &str) -> Option<PathBuf> {
    if dir_part.is_empty() {
        current_dir().ok()
    } else {
        let path = Path::new(dir_part);
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            current_dir().ok().map(|cwd| cwd.join(path))
        }
    }
}

fn longest_common_prefix(names: &[&str]) -> String {
    if names.is_empty() {
        return String::new();
    }
    let mut prefix = names[0].to_string();
    for name in &names[1..] {
        let mut common = String::new();
        for (a, b) in prefix.chars().zip(name.chars()) {
            if a == b {
                common.push(a);
            } else {
                break;
            }
        }
        prefix = common;
        if prefix.is_empty() {
            break;
        }
    }
    prefix
}
