use std::{
    env::current_dir,
    io::{Write, stdout},
    process::exit,
    sync::{Arc, Mutex},
};

use crate::{
    input::wait_keys,
    output::{
        ansi::util::{clear, cursor_back, cursor_down, cursor_right, newline, scroll_up},
        prompt::get_prompt,
        term_mode::{set_origin_term, set_raw_term},
        term_size::read_terminal_size,
    },
    shell::{
        Shell,
        line_edit_mode::key_function::KeyFunction,
        pipeline::{
            execute::execute,
            parse::parse,
            pre_parse::pre_parse,
            tokenize::{tokenize, tokens_to_string},
        },
    },
};

pub mod key_function;

pub fn line_edit_mode(shell: &Arc<Mutex<Shell>>) {
    set_raw_term();
    print_prompt();

    let mut buffer = String::new();
    let mut cursor = 0;
    let mut pre_cursor = cursor;
    loop {
        let keys = match wait_keys(10) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let key_functions = keys.into_iter().map(|x| x.function());

        for f in key_functions.flatten() {
            apply(f, shell, &mut buffer, &mut cursor);
        }
        delete_pre_buffer(pre_cursor);
        pre_cursor = cursor;
        print_buffer_cursor(&buffer, cursor);
        flush();
    }
}

fn print_prompt() {
    write!(stdout().lock(), "{}", get_prompt()).unwrap();
}

fn delete_pre_buffer(cursor: usize) {
    write!(
        stdout().lock(),
        "{}",
        crate::output::ansi::util::delete_buffer(cursor)
    )
    .unwrap();
}

fn print_buffer_cursor(buffer: &str, cursor: usize) {
    let width = read_terminal_size().width;
    let mut out = stdout().lock();
    write!(out, "{}", buffer).unwrap();
    if buffer.len() % width as usize == 0 && buffer.len() != 0 {
        write!(out, "{}", scroll_up(1)).unwrap();
    }
    write!(out, "{}", cursor_back(buffer.len())).unwrap();
    write!(out, "{}", cursor_down(cursor as u32 / width as u32)).unwrap();
    write!(out, "{}", cursor_right(cursor as u32 % width as u32)).unwrap();
}

fn print_newline() {
    write!(stdout().lock(), "{}", newline()).unwrap();
}

fn print_clear() {
    write!(stdout().lock(), "{}", clear()).unwrap();
}

fn flush() {
    stdout().lock().flush().unwrap();
}

fn apply(
    key_function: KeyFunction,
    shell: &Arc<Mutex<Shell>>,
    buffer: &mut String,
    cursor: &mut usize,
) {
    match key_function {
        KeyFunction::Char(c) => {
            buffer.insert(*cursor, c);
            *cursor += 1;
        }
        KeyFunction::PreCmd => {
            *buffer = shell.lock().unwrap().history.prev(&buffer);
            *cursor = buffer.len();
        }
        KeyFunction::NextCmd => {
            *buffer = shell.lock().unwrap().history.next();
            *cursor = buffer.len();
        }
        KeyFunction::Left => {
            if *cursor > 0 {
                *cursor -= 1;
            }
        }
        KeyFunction::Right => {
            if *cursor < buffer.len() {
                *cursor += 1;
            }
        }
        KeyFunction::Tab => {}
        KeyFunction::Home => {
            *cursor = 0;
        }
        KeyFunction::End => {
            *cursor = buffer.len();
        }
        KeyFunction::Enter => {
            let tokens = pre_parse(tokenize(&buffer), shell);
            let new_buffer = tokens_to_string(&tokens);
            if *buffer != new_buffer {
                *buffer = new_buffer;
                delete_pre_buffer(*cursor);
                print_buffer_cursor(buffer, *cursor);
            }
            let Some(parsed) = parse(&tokens) else {
                return;
            };
            let parsed = super::pipeline::pre_execute::expand_expr_with_shell(&parsed.0, &shell);
            print_newline();
            set_origin_term();
            let _ = execute(&parsed, shell);
            let pwd = current_dir().unwrap();
            shell
                .lock()
                .unwrap()
                .history
                .push(pwd.to_string_lossy().to_string(), buffer.clone());
            set_raw_term();
            print_prompt();
            buffer.clear();
            *cursor = 0;
        }
        KeyFunction::BackSpace => {
            if *cursor > 0 {
                buffer.remove(*cursor - 1);
                *cursor -= 1;
            }
        }
        KeyFunction::Delete => {
            if *cursor < buffer.len() {
                buffer.remove(*cursor);
            }
        }
        KeyFunction::Clear => {
            print_clear();
            print_prompt();
            print_buffer_cursor(&buffer, *cursor);
        }
        KeyFunction::DeleteWord => {
            if buffer.is_empty() || *cursor == 0 {
                // 何もしない
            } else {
                // まず1文字削除（カーソル直前）
                let r = *cursor;
                let mut target = buffer[0..r].chars();
                let mut l = r - 1;
                target.next_back().unwrap();

                while l > 0 {
                    let next = target.next_back().unwrap();
                    if !"/ ".contains(next) {
                        l -= 1;
                    } else {
                        break;
                    }
                }
                buffer.replace_range(l..r, "");
                *cursor -= r - l;
            }
        }
        KeyFunction::Ctrl('d') => {
            if buffer.is_empty() {
                shell.lock().unwrap().history.store();
                exit(0);
            }
            if *cursor < buffer.len() {
                buffer.remove(*cursor);
            }
        }
        KeyFunction::Ctrl('c') => {
            if !buffer.is_empty() {
                delete_pre_buffer(*cursor);
                print_buffer_cursor(buffer, *cursor);
                print_newline();
                print_prompt();
            }
            buffer.clear();
            *cursor = 0;
        }
        _ => (),
    }
}
