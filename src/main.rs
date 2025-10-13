mod error;
mod pipeline;
mod shell;
mod ui;

use error::Result;
use shell::Shell;
use ui::{Action, Mode};

use crate::{
    pipeline::{execute, expand_aliases, parse, tokenize, tokens_to_string},
    ui::{
        clean_term, delete_printing, flush, init, print_command_line, print_newline, print_prompt,
        set_origin_term, set_raw_term, wait_actions,
    },
};

fn main() -> Result<()> {
    init();
    let mut shell = Shell::new();
    let mut buffer = String::new();
    let mut cursor = 0;
    set_raw_term();
    print_prompt();
    'finish: loop {
        delete_printing(cursor);
        print_command_line(&buffer, cursor, &shell.get_ghost(&buffer));
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
                    unimplemented!()
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
        let c = buffer.chars().nth(l).unwrap();
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
