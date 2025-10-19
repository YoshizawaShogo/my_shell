mod error;
mod pipeline;
mod shell;
mod ui;

use std::{
    borrow::Cow, collections::BTreeSet, fs, io, path::{Path, PathBuf, MAIN_SEPARATOR}
};

use error::Result;
use shell::Shell;
use ui::{Action, Mode};

use crate::{
    pipeline::{execute, expand_aliases, parse, tokenize, tokens_to_string},
    ui::{
        clean_term,  delete_printing, flush, init, print_candidates,
        print_command_line, print_hat_c, print_newline, print_prompt, read_terminal_size,
        set_origin_term, set_raw_term, wait_actions,
    },
};

fn main() -> Result<()> {
    init();
    let mut shell = Shell::new();
    let mut buffer = String::new();
    let mut cursor = 0;
    let mut candidates = vec![];
    let mut completion_fixed_len = 0;
    let mut pre_action = Action::None;
    set_raw_term();
    print_prompt();
    'finish: loop {
        delete_printing(cursor);
        print_command_line(&buffer, cursor, &shell.get_ghost(&buffer));
        if pre_action == Action::Tab {
            print_candidates(&candidates, cursor, None, completion_fixed_len);
        }
        flush();
        let Ok(actions) = wait_actions(&Mode::LineEdit, 20) else {
            continue;
        };
        for action in actions {
            match action {
                Action::Char(c) => {
                    buffer.insert(cursor, c);
                    cursor += 1;
                }
                Action::Space => {
                    expand_abbr(&mut buffer, &mut cursor, &shell);
                    buffer.insert(cursor, ' ');
                    cursor += 1;
                }
                Action::Ctrl('c') => {
                    reset(&mut buffer, &mut cursor, &mut shell);
                }
                Action::Ctrl('d') => {
                    if buffer.is_empty() {
                        break 'finish;
                    }
                    if cursor < buffer.len() {
                        buffer.remove(cursor);
                    }
                }
                Action::Ctrl('r') => {
                    buffer = shell.history.prev_r(&buffer);
                    cursor = buffer.len();
                }
                Action::PreCmd => {
                    buffer = shell.history.prev_up(&buffer);
                    cursor = buffer.len();
                }
                Action::NextCmd => {
                    buffer = shell.history.next_down();
                    cursor = buffer.len();
                }
                Action::Left => {
                    cursor = cursor.saturating_sub(1);
                }
                Action::Right => {
                    if cursor < buffer.len() {
                        cursor += 1;
                    } else {
                        buffer += &shell.get_ghost(&buffer);
                        cursor = buffer.len();
                    }
                }
                Action::Tab => {
                    if pre_action == Action::Tab && candidates.len() >= 2 {
                        complete_mode(&mut buffer, &mut cursor, &candidates, completion_fixed_len, &mut shell);
                        candidates = vec![];
                    } else {
                        (candidates, completion_fixed_len) =
                            complete(&mut buffer, &mut cursor, &mut shell);
                    }
                }
                Action::Home => cursor = 0,
                Action::End => cursor = buffer.len(),
                Action::Enter => {
                    let pre_cursor = cursor;
                    expand_abbr(&mut buffer, &mut cursor, &shell);
                    delete_printing(pre_cursor);
                    print_command_line(&buffer, cursor, "");
                    if buffer.is_empty() {
                        print_prompt();
                    } else {
                        run_pipeline(&mut shell, &mut buffer, &mut cursor)
                    }
                }
                Action::BackSpace => {
                    if cursor > 0 {
                        buffer.remove(cursor - 1);
                        cursor -= 1;
                    }
                }
                Action::Delete => {
                    if cursor < buffer.len() {
                        buffer.remove(cursor);
                    }
                }
                Action::Clear => {
                    clean_term();
                    print_prompt();
                }
                Action::DeleteWord => delete_word(&mut buffer, &mut cursor),
                _ => {}
            }
            pre_action = action;
        }
    }

    set_origin_term();
    Ok(())
}

fn run_pipeline(shell: &mut Shell, buffer: &mut String, cursor: &mut usize) {
    let tokens = tokenize(buffer);
    *buffer = tokens_to_string(&tokens);
    let tokens = expand_aliases(tokens, shell);
    let expr = match parse(&tokens) {
        Ok(expr) => expr,
        Err(e) => {
            print_newline();
            print!("{}", e);
            print_newline();
            print_prompt();
            return;
        }
    };
    print_newline();
    set_origin_term();
    shell.history.push(buffer.clone());
    let _ = execute(&expr, shell);
    set_raw_term();
    print_prompt();
    buffer.clear();
    *cursor = 0;
}

fn delete_word(buffer: &mut String, cursor: &mut usize) {
    if buffer.is_empty() || *cursor == 0 {
        return;
    }
    let r = *cursor;
    let mut l = r - 1;
    while l > 0 {
        let c = buffer.chars().nth(l - 1).unwrap();
        if !"/ ".contains(c) {
            l -= 1;
        } else {
            break;
        }
    }
    buffer.replace_range(l..r, "");
    *cursor -= r - l;
}

fn expand_abbr(buffer: &mut String, cursor: &mut usize, shell: &Shell) -> bool {
    if buffer.ends_with(' ') {
        return false;
    }
    let old_len = buffer.len();
    let tokens = tokenize(buffer);
    if let Some(expanded) = crate::pipeline::expand_abbr(tokens, shell) {
        *buffer = tokens_to_string(&expanded);
        let new_len = buffer.len();
        *cursor += new_len - old_len;
        true
    } else {
        false
    }
}

fn complete_mode(
    buffer: &mut String,
    cursor: &mut usize,
    candidates: &Vec<String>,
    fixed_len: usize,
    shell: &mut Shell
) {
    let mut index = 0;
    'finish: loop {
        delete_printing(*cursor);
        let adder = &candidates[index][fixed_len..];
        let mut tmp_buffer = buffer.clone() + adder;
        let mut tmp_cursor = *cursor + adder.len();
        if !tmp_buffer.ends_with("/") {
            tmp_buffer += " ";
            tmp_cursor += 1;
        }
        print_command_line(&tmp_buffer, tmp_cursor, "");
        let width = print_candidates(&candidates, tmp_cursor, Some(index), fixed_len);
        flush();
        let Ok(actions) = wait_actions(&Mode::Completion, 20) else {
            continue;
        };
        for action in actions {
            match action {
                Action::Up => {
                    if index > width {
                        index -= width;
                    }
                }
                Action::Down => {
                    if index + width < candidates.len() {
                        index += width;
                    }
                }
                Action::Left => {
                    if index == 0 {
                        index = candidates.len() - 1;
                    } else {
                        index -= 1;
                    }
                }
                Action::Right => {
                    if index + 1 == candidates.len() {
                        index = 0;
                    } else {
                        index += 1;
                    }
                }
                Action::Char('c') => {
                    reset(buffer, cursor, shell);
                    return;
                }
                _ => break 'finish,
            }
        }
    }
    let adder = &candidates[index][fixed_len..];
    *buffer += adder;
    *cursor += adder.len();
    if !buffer.ends_with("/") {
        buffer.insert(*cursor, ' ');
        *cursor += 1;
    }
}

fn complete(buffer: &mut String, cursor: &mut usize, shell: &mut Shell) -> (Vec<String>, usize) {
    let last_is_space = buffer[..*cursor].ends_with(' ');
    let tokens = tokenize(&buffer[..*cursor]);
    let Ok(expr) = parse(&tokens) else {
        return (vec![], 0);
    };
    let last_cmd = expr.last_cmd_expr();
    let cmd = last_cmd.cmd_name.concat_text(shell);
    let args: Vec<String> = last_cmd
        .args
        .into_iter()
        .map(|x| x.concat_text(shell))
        .collect();
    if &cmd == "cd" {
        return complete_cd(buffer, cursor, &args, last_is_space);
    }

    let cmd_completion = shell.completion.data.get(&cmd);
    let subcmd_completion = cmd_completion.map(|x| &x.subcommands);
    let is_option = args.last().is_some_and(|x| x.starts_with("-"));

    // cmd [sub] [file|option]* しか考慮しない。
    match (args.len(), last_is_space, subcmd_completion, is_option) {
        (0, false, ..) => {
            // 現在、cmdを書いている途中。
            // /を含んでいる場合はfile補完
            // そうでなければ、cmdやaliasなどを補完
            if cmd.contains("/") {
                let (dir, file) = completion_split(&cmd);
                let src = get_exes(&dir);
                return complete_parts(src, &file, buffer, cursor);
            } else {
                let src = shell.exe_list.command_candidates(&cmd);
                return complete_parts(src, &cmd, buffer, cursor);
            }
        }
        (0, true, Some(sub_cmp), _) => {
            // 現在、cmdをちょうど書き終えたところ。
            // subcmdかfileかオプションを書き始めるところ。
            // subがあればsub
            let src = sub_cmp.keys().cloned();
            return complete_parts(src, "", buffer, cursor);
        }
        (0, true, None, _) => {
            // そうでなければ、file
            let src = get_files(".");
            return complete_parts(src, "", buffer, cursor);
        }
        (1, false, _, true) => {
            // 一つ目の引数を書いている途中
            // -から始まっているとoption補完
            if cmd_completion.is_none() {
                return (vec![], 0);
            }
            let last_arg = args.last().unwrap();
            let src = cmd_completion.unwrap().options.clone();
            return complete_parts(src, last_arg, buffer, cursor);
        }
        (1, false, Some(sub_cmp), false) => {
            // subがあればsub
            let last_arg = args.last().unwrap();
            let src = sub_cmp.keys().cloned();
            return complete_parts(src, last_arg, buffer, cursor);
        }
        (1, false, None, false) => {
            // そうでなければ、file
            let last_arg = args.last().unwrap();
            let (dir, file) = completion_split(&last_arg);
            let src = get_files(&dir);
            return complete_parts(src, &file, buffer, cursor);
        }
        (_, true, _, _) => {
            // 一つ目の引数を書き終えたところ。
            // file補完一択
            let src = get_files(".");
            return complete_parts(src, "", buffer, cursor);
        }
        (_, false, Some(sub_cmp), true) => {
            // -から始まっていると、option補完
            // 引数1がサブコマンドであれば、option補完内容が変わるので注意。
            let sub_cmd = args.first().unwrap();
            let last_arg = args.last().unwrap();
            let options = sub_cmp.get(sub_cmd).map_or(BTreeSet::default(), |x| x.options.clone());
            return complete_parts(options, last_arg, buffer, cursor);
        }
        (_, false, None, true) => {
            // -から始まっていると、option補完
            if cmd_completion.is_none() {
                return (vec![], 0);
            }
            let last_arg = args.last().unwrap();
            let src = cmd_completion.unwrap().options.clone();
            return complete_parts(src, last_arg, buffer, cursor)
        }
        (_, false, _, false) => {
            // それ以外はfile補完
            let last_arg = args.last().unwrap();
            let (dir, file) = completion_split(&last_arg);
            let src = get_files(&dir);
            return complete_parts(src, &file, buffer, cursor);
        }
    };
}

fn list_with<F>(path: &str, filter: F) -> Vec<String>
where
    F: Fn(&fs::DirEntry) -> bool,
{
    let mut v: Vec<_> = fs::read_dir(path)
        .ok()
        .into_iter()
        .flat_map(|it| it.flatten())
        .filter(|e| filter(e))
        .map(|e| {
            let mut name = e.file_name().to_string_lossy().into_owned();
            if let Ok(ft) = fs::metadata(Path::new(path).join(&name)) {
                if ft.is_dir() && !name.ends_with('/') {
                    name.push('/');
                }
            }
            name
        })
        .collect();
    v.sort_unstable();
    v
}

pub fn get_files(path: &str) -> Vec<String> {
    list_with(
        path,
        |_| true, // ここでは全件（必要なら is_file 判定に変更）
    )
}

pub fn get_dirs(path: &str) -> Vec<String> {
    list_with(path, |e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
}

use std::os::unix::fs::PermissionsExt;
pub fn get_exes(path: &str) -> Vec<String> {
    list_with(path, |e| {
        e.metadata()
            .ok()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    })
}

pub fn complete_parts(
    src: impl IntoIterator<Item = String>,
    prefix: &str,
    buffer: &mut String,
    cursor: &mut usize,
) -> (Vec<String>, usize) {
    let candidates: Vec<String> = src.into_iter().filter(|x| x.starts_with(prefix)).collect();
    if candidates.is_empty() {
        return (vec![], 0);
    }
    let common = common_prefix(candidates.iter().cloned());
    let adder = common[prefix.len()..].to_string();
    buffer.insert_str(*cursor, &adder);
    *cursor += adder.len();

    if candidates.len() == 1 {
        if !buffer.ends_with("/") {
            buffer.insert(*cursor, ' ');
            *cursor += 1;
        }
    }

    (candidates, common.len())
}

fn complete_cd(
    buffer: &mut String,
    cursor: &mut usize,
    args: &Vec<String>,
    last_is_space: bool,
) -> (Vec<String>, usize) {
    if args.len() >= 2 || (args.len() == 1 && last_is_space) {
        return (vec![], 0);
    }
    let last_word = args.last().cloned().unwrap_or_default();
    let (dir, file) = completion_split(&last_word);
    let src = get_dirs(&dir);
    return complete_parts(src, &file, buffer, cursor);
}

fn completion_split(input: &str) -> (String, String) {
    match input.rfind(MAIN_SEPARATOR) {
        Some(pos) if pos == 0 => ("/".to_string(), input[1..].to_string()),
        Some(pos) => {
            let dir = &input[..pos];
            let file = input[pos + 1..].to_string();

            let dir = match dir {
                "." => "./".to_string(),
                _ if dir.starts_with('/') || dir.starts_with('.') => dir.to_string(),
                _ => format!("./{dir}"),
            };

            (dir, file)
        }
        None => {
            let dir = "./".to_string();
            let file = input.to_string();
            (dir, file)
        }
    }
}

fn common_prefix<I>(mut strings: I) -> String
where
    I: Iterator<Item = String>,
{
    // 最初の要素を取得（なければ空文字を返す）
    let Some(first) = strings.next() else {
        return String::new();
    };

    let mut prefix = first;

    for s in strings {
        let mut i = 0;
        for (a, b) in prefix.chars().zip(s.chars()) {
            if a != b {
                break;
            }
            i += a.len_utf8();
        }
        prefix.truncate(i);
        if prefix.is_empty() {
            break;
        }
    }

    prefix
}

fn reset(buffer: &mut String, cursor: &mut usize, shell: &mut Shell) {
    if !buffer.is_empty() {
        print_hat_c();
        print_newline();
        print_prompt();
    }
    shell.history.index_up = 0;
    shell.history.index_r = 0;
    buffer.clear();
    *cursor = 0;
}
