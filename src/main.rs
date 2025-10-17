mod error;
mod pipeline;
mod shell;
mod ui;

use std::{
    borrow::Cow,
    fs,
    path::{MAIN_SEPARATOR, Path, PathBuf},
};

use error::Result;
use shell::Shell;
use ui::{Action, Mode};

use crate::{
    pipeline::{execute, expand_aliases, parse, tokenize, tokens_to_string},
    ui::{
        clean_term, delete_printing, flush, init, print_candidates, print_command_line,
        print_newline, print_prompt, read_terminal_size, set_origin_term, set_raw_term,
        wait_actions,
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
        print_candidates(&candidates, cursor, None, completion_fixed_len);
        flush();
        candidates = vec![];
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
                    if !buffer.is_empty() {
                        print_newline();
                        print_prompt();
                    }
                    shell.history.index = 0;
                    buffer.clear();
                    cursor = 0;
                }
                Action::Ctrl('d') => {
                    if buffer.is_empty() {
                        break 'finish;
                    }
                    if cursor < buffer.len() {
                        buffer.remove(cursor);
                    }
                }
                Action::PreCmd => {
                    buffer = shell.history.prev(&buffer);
                    cursor = buffer.len();
                }
                Action::NextCmd => {
                    buffer = shell.history.next();
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
                        // completion_mode();
                    } else {
                        (candidates, completion_fixed_len) = complete(&mut buffer, &mut cursor, &shell);
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

// fn complete_mode(buffer: &mut String, cursor: &mut usize, shell: &Shell) {
//     let mut candidates = vec![];
//     let mut index = None;
//     'finish: loop {
//         delete_printing(*cursor);
//         print_command_line(buffer, *cursor, &shell.get_ghost(&buffer));
//         print_candidates(&candidates, *cursor, index);
//         flush();
//         let Ok(actions) = wait_actions(&Mode::Completion, 20) else {
//             continue;
//         };
//         for action in actions {
//             match action {
//                 Action::Char(c) => {
//                     buffer.insert(*cursor, c);
//                     *cursor += 1;
//                 }
//                 Action::Space => {
//                     expand_abbr(&mut buffer, &mut cursor, &shell);
//                     buffer.insert(*cursor, ' ');
//                     *cursor += 1;
//                 }
//                 Action::Ctrl('c') => {
//                     if !buffer.is_empty() {
//                         print_newline();
//                         print_prompt();
//                     }
//                     shell.history.index = 0;
//                     buffer.clear();
//                     *cursor = 0;
//                     return;
//                 }
//                 Action::Up => {
//                     let width = read_terminal_size().width as usize;
//                     index = Some(match index {
//                         Some(i) if i >= width => i - width,
//                         Some(i) => i,
//                         None => candidates.len().saturating_sub(1),
//                     });
//                 }
//                 Action::Down => {
//                     let width = read_terminal_size().width as usize;
//                     index = Some(match index {
//                         Some(i) if i + width < candidates.len() => i + width,
//                         Some(i) => i,
//                         None => candidates.len().saturating_sub(1),
//                     });
//                 }
//                 Action::NextCmd => {
//                     buffer = shell.history.next();
//                     cursor = buffer.len();
//                 }
//                 Action::Left => {
//                     cursor = cursor.saturating_sub(1);
//                 }
//                 Action::Right => {
//                     if cursor < buffer.len() {
//                         cursor += 1;
//                     } else {
//                         buffer += &shell.get_ghost(&buffer);
//                         cursor = buffer.len();
//                     }
//                 }
//                 Action::Tab => {
//                     complete(&mut buffer, &mut cursor, &shell);
//                 }
//                 Action::Home => cursor = 0,
//                 Action::End => cursor = buffer.len(),
//                 Action::Enter => {
//                     let pre_cursor = cursor;
//                     expand_abbr(&mut buffer, &mut cursor, &shell);
//                     delete_printing(pre_cursor);
//                     print_command_line(&buffer, cursor, "");
//                     if buffer.is_empty() {
//                         print_prompt();
//                     } else {
//                         run_pipeline(&mut shell, &mut buffer, &mut cursor)
//                     }
//                 }
//                 Action::BackSpace => {
//                     if cursor > 0 {
//                         buffer.remove(cursor - 1);
//                         cursor -= 1;
//                     }
//                 }
//                 Action::Delete => {
//                     if cursor < buffer.len() {
//                         buffer.remove(cursor);
//                     }
//                 }
//                 Action::Clear => {
//                     clean_term();
//                     print_prompt();
//                 }
//                 Action::DeleteWord => delete_word(&mut buffer, &mut cursor),
//                 _ => {}
//             }
//         }
//     }
// }

fn complete(buffer: &mut String, cursor: &mut usize, shell: &Shell) -> (Vec<String>, usize) {
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

    // cmd [sub] [file|option]* しか考慮しない。
    match (args.len(), last_is_space) {
        (0, false) => {
            // 現在、cmdを書いている途中。
            // /を含んでいる場合はfile補完
            // そうでなければ、cmdやaliasなどを補完
        }
        (0, true) => {
            // 現在、cmdをちょうど書き終えたところ。
            // subcmdかfileかオプションを書き始めるところ。
            // subがあればsub
            // そうでなければ、file
        }
        (1, false) => {
            // 一つ目の引数を書いている途中
            // -から始まっているとoption補完
            // subがあればsub
            // そうでなければ、file
        }
        (_, true) => {
            // 一つ目の引数を書き終えたところ。
            // file補完一択
        }
        (_, false) => {
            // -から始まっていると、option補完
            // 引数1がサブコマンドであれば、option補完内容が変わるので注意。
            // それ以外はfile補完
        }
    };
    return (vec![], 0);
}

fn get_dirs(path: &Path) -> Vec<String> {
    let mut dirs: Vec<String> = match fs::read_dir(path) {
        Ok(entries) => entries
            .flatten()
            .filter(|x| x.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|x| x.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => vec![],
    };
    dirs.sort_unstable();
    dirs
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
    let last_word = args.last();
    let (dir, file) = completion_split(last_word);
    let candidates: Vec<String> = get_dirs(Path::new(&dir))
        .iter()
        .filter(|x| x.starts_with(&file))
        .cloned()
        .collect();
    if candidates.len() == 0 {
        return (vec![], 0);
    } else if candidates.len() == 1 {
        let adder = candidates.first().unwrap()[file.len()..].to_string() + "/";
        buffer.insert_str(*cursor, &adder);
        *cursor += adder.len();
        return (vec![], 0);
    } else {
        return (candidates, file.len());
    }
}

pub fn completion_split(input: Option<&String>) -> (String, String) {
    let input = match input {
        Some(x) => x,
        None => return ("./".to_string(), "".to_string()),
    };
    match input.rfind(MAIN_SEPARATOR) {
        Some(pos) if pos == 0 => {
            return ("/".to_string(), input[1..].to_string());
        }
        Some(pos) => {
            let mut dir = input[..pos].to_string();
            let file = input[pos + 1..].to_string();
            if dir.starts_with("/") || dir.starts_with(".") {
                return (dir, file);
            }
            dir = "./".to_string() + &dir;
            return (dir, file);
        }
        None => {
            let dir = "./".to_string();
            let file = input.to_string();
            return (dir, file);
        }
    };
}
