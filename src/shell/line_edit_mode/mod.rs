use std::{
    env::current_dir,
    io::{Read, Write, stdout},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
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
        completion::commands_from_expr,
        line_edit_mode::key_function::KeyFunction,
        pipeline::{
            execute::execute,
            parse::parse,
            pre_parse::expand_aliases,
            tokenize::{tokenize, tokens_to_string},
        },
        tab_completion_mode::{DisplayEntry, tab_completion_mode},
    },
};

pub mod key_function;

pub fn line_edit_mode(shell: &Arc<Mutex<Shell>>) {
    set_raw_term();
    print_prompt();

    let mut buffer = String::new();
    let mut cursor = 0;
    let mut pending_tab_list: Option<Vec<DisplayEntry>> = None;
    let mut last_key_was_tab = false;
    let mut last_display_len = 0;

    'main_loop: loop {
        let keys = match wait_keys(10) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let key_functions = keys.into_iter().map(|x| x.edit_function());

        let mut tab_display: Option<Vec<DisplayEntry>> = None;
        let mut exit_requested = false;
        let mut ghost_override: Option<Option<String>> = None;

        for key_func in key_functions.flatten() {
            let is_tab = matches!(key_func, KeyFunction::Tab);

            if is_tab && last_key_was_tab {
                if let Some(list) = pending_tab_list.take() {
                    tab_display = Some(list);
                    last_key_was_tab = false;
                    continue;
                }
            }

            let before_buffer = buffer.clone();
            let before_cursor = cursor;

            match apply(key_func, shell, &mut buffer, &mut cursor) {
                ApplyResult::Tab(display) => {
                    ghost_override = Some(None);
                    if display.is_empty() {
                        pending_tab_list = None;
                        last_key_was_tab = false;
                        continue;
                    }
                    if is_tab {
                        tab_display = Some(display.clone());
                        pending_tab_list = Some(display);
                        last_key_was_tab = true;
                        continue;
                    } else {
                        tab_display = Some(display);
                    }
                }
                ApplyResult::Ghost(opt) => {
                    ghost_override = Some(opt);
                    continue;
                }
                ApplyResult::Exit => {
                    exit_requested = true;
                    pending_tab_list = None;
                    last_key_was_tab = false;
                    break;
                }
                ApplyResult::None => {}
            }

            let changed = buffer != before_buffer || cursor != before_cursor;
            if is_tab {
                if changed {
                    pending_tab_list = None;
                    last_key_was_tab = false;
                } else {
                    last_key_was_tab = pending_tab_list.is_some();
                }
            } else {
                pending_tab_list = None;
                last_key_was_tab = false;
            }
        }

        if exit_requested {
            break 'main_loop;
        }

        let new_ghost = ghost_override.unwrap_or_else(|| {
            if cursor == buffer.len() {
                history_suggestion(shell, &buffer)
            } else {
                None
            }
        });

        if let Some(display) = tab_display {
            set_origin_term();
            if !display.is_empty() {
                last_display_len = print_tab_candidates(&display, &buffer, cursor);
            } else {
                delete_pre_buffer(last_display_len);
                last_display_len = print_buffer_cursor(&buffer, cursor, None);
            }
            set_raw_term();
            pending_tab_list = None;
            last_key_was_tab = false;
        } else {
            delete_pre_buffer(last_display_len);
            last_display_len = print_buffer_cursor(&buffer, cursor, new_ghost.as_deref());
        }
        flush();
    }
    set_origin_term();
    print_newline();
    flush();
}

fn print_prompt() {
    write!(stdout().lock(), "{}", get_prompt()).unwrap();
}

fn delete_pre_buffer(display_len: usize) {
    if display_len == 0 {
        return;
    }
    write!(
        stdout().lock(),
        "{}",
        crate::output::ansi::util::delete_buffer(display_len)
    )
    .unwrap();
}

fn print_buffer_cursor(buffer: &str, cursor: usize, ghost: Option<&str>) -> usize {
    let width = read_terminal_size().width;
    let mut out = stdout().lock();
    write!(out, "{}", buffer).unwrap();
    let ghost_len = ghost.map(|g| g.len()).unwrap_or(0);
    if let Some(text) = ghost {
        write!(out, "\x1b[90m{}\x1b[0m", text).unwrap();
    }
    let display_len = buffer.len() + ghost_len;
    if display_len % width as usize == 0 && display_len != 0 {
        write!(out, "{}", scroll_up(1)).unwrap();
    }
    write!(out, "{}", cursor_back(display_len)).unwrap();
    write!(out, "{}", cursor_down(cursor as u32 / width as u32)).unwrap();
    write!(out, "{}", cursor_right(cursor as u32 % width as u32)).unwrap();
    display_len
}

fn print_newline() {
    write!(stdout().lock(), "{}", newline()).unwrap();
}

fn print_clear() {
    write!(stdout().lock(), "{}", clear()).unwrap();
}

fn print_tab_candidates(candidates: &[DisplayEntry], buffer: &str, cursor: usize) -> usize {
    if candidates.is_empty() {
        return 0;
    }
    print_newline();
    {
        let mut out = stdout().lock();
        let width = read_terminal_size().width as usize;
        let max_len = candidates
            .iter()
            .map(|c| c.plain.chars().count())
            .max()
            .unwrap_or(0);
        let col_width = max_len.saturating_add(2).min(width.max(1));
        let cols = if col_width == 0 {
            1
        } else {
            (width / col_width).max(1)
        };

        for (idx, candidate) in candidates.iter().enumerate() {
            if cols == 1 {
                writeln!(out, "{}", candidate.styled).unwrap();
                continue;
            }
            let is_row_end = (idx + 1) % cols == 0;
            if is_row_end {
                writeln!(out, "{}", candidate.styled).unwrap();
            } else {
                let plain_len = candidate.plain.chars().count();
                let padding = col_width.saturating_sub(plain_len);
                write!(out, "{}{}", candidate.styled, " ".repeat(padding)).unwrap();
            }
        }
        if cols > 1 && candidates.len() % cols != 0 {
            writeln!(out).unwrap();
        }
    }
    print_newline();
    print_prompt();
    print_buffer_cursor(buffer, cursor, None)
}

fn flush() {
    stdout().lock().flush().unwrap();
}

enum ApplyResult {
    None,
    Tab(Vec<DisplayEntry>),
    Ghost(Option<String>),
    Exit,
}

fn capture_help_output(tokens: &[String]) -> Option<String> {
    if tokens.is_empty() {
        return None;
    }
    let command = &tokens[0];
    if crate::shell::builtins::find(command.as_str()).is_some() {
        return None;
    }
    let mut cmd = Command::new(command);
    if tokens.len() > 1 {
        cmd.args(&tokens[1..]);
    }
    cmd.env("PAGER", "cat")
        .env("MANPAGER", "cat")
        .env("GIT_PAGER", "cat")
        .env("SYSTEMD_PAGER", "cat")
        .env("HELP_PAGER", "cat")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().ok()?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_handle = stdout.map(|mut out| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = out.read_to_end(&mut buf);
            buf
        })
    });
    let stderr_handle = stderr.map(|mut err| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = err.read_to_end(&mut buf);
            buf
        })
    });

    let timeout = Duration::from_millis(500);
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                break;
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }

    let stdout_buf = stdout_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let stderr_buf = stderr_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();

    let mut text = String::from_utf8_lossy(&stdout_buf).into_owned();
    let stderr_text = String::from_utf8_lossy(&stderr_buf);
    if text.trim().is_empty() && !stderr_text.trim().is_empty() {
        text = stderr_text.into_owned();
    } else if !stderr_text.trim().is_empty() {
        text.push('\n');
        text.push_str(stderr_text.trim());
    }
    if text.trim().is_empty() {
        return None;
    }
    Some(text)
}

fn apply(
    key_function: KeyFunction,
    shell: &Arc<Mutex<Shell>>,
    buffer: &mut String,
    cursor: &mut usize,
) -> ApplyResult {
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
        KeyFunction::Right | KeyFunction::Ctrl('f') => {
            let suggestion = history_suggestion(shell, buffer);
            if *cursor < buffer.len() {
                *cursor += 1;
                return ApplyResult::Ghost(suggestion);
            } else if let Some(suffix) = suggestion {
                buffer.push_str(&suffix);
                *cursor = buffer.len();
                return ApplyResult::Ghost(None);
            }
            return ApplyResult::Ghost(None);
        }
        KeyFunction::Tab => {
            if let Some(display) = tab_completion_mode(shell, buffer, cursor) {
                return ApplyResult::Tab(display);
            }
            return ApplyResult::None;
        }
        KeyFunction::Home => {
            *cursor = 0;
        }
        KeyFunction::End => {
            *cursor = buffer.len();
        }
        KeyFunction::Enter => {
            let ghost_len = history_suggestion(shell, buffer)
                .map(|s| s.len())
                .unwrap_or(0);
            delete_pre_buffer(buffer.len() + ghost_len);
            print_buffer_cursor(buffer, *cursor, None);
            expand_abbr(shell, buffer, cursor);
            *buffer = buffer.trim().to_string();
            let tokens = expand_aliases(tokenize(&buffer), shell);
            let Some(parsed) = parse(&tokens) else {
                return ApplyResult::None;
            };
            let parsed = super::pipeline::pre_execute::expand_expr_with_shell(&parsed.0, &shell);
            let commands_for_help = commands_from_expr(&parsed);
            print_newline();
            set_origin_term();
            let execute_result = execute(&parsed, shell);
            let pwd = current_dir().unwrap();
            shell
                .lock()
                .unwrap()
                .history
                .push(pwd.to_string_lossy().to_string(), buffer.clone());
            if matches!(execute_result, Ok(code) if code == 0) {
                let mut help_data = Vec::new();
                for tokens in &commands_for_help {
                    if tokens.iter().any(|t| t == "--help") {
                        if let Some(text) = capture_help_output(tokens) {
                            help_data.push((tokens.clone(), text));
                        }
                    }
                }
                if let Ok(mut sh) = shell.lock() {
                    sh.record_completion_from_expr(&parsed);
                    for (tokens, text) in help_data {
                        sh.completion.record_help_output(&tokens, &text);
                    }
                }
            }
            if matches!(shell.lock().map(|sh| sh.exit_requested), Ok(true)) {
                return ApplyResult::Exit;
            }
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
                if let Ok(sh) = shell.lock() {
                    sh.history.store();
                }
                return ApplyResult::Exit;
            }
            if *cursor < buffer.len() {
                buffer.remove(*cursor);
            }
        }
        KeyFunction::Ctrl('c') => {
            if !buffer.is_empty() {
                print_newline();
                print_prompt();
            }
            buffer.clear();
            *cursor = 0;
        }
        KeyFunction::Ctrl(_) => {}
        KeyFunction::Space => {
            expand_abbr(shell, buffer, cursor);
            buffer.insert(*cursor, ' ');
            *cursor += 1;
        }
    }
    ApplyResult::None
}

fn expand_abbr(shell: &Arc<Mutex<Shell>>, buffer: &mut String, cursor: &mut usize) {
    let target = buffer[0..*cursor].to_string();
    let tail = &buffer[*cursor..];
    let tokens = tokenize(&target);
    let expand_abbr_tokens = crate::shell::pipeline::pre_parse::expand_abbr(tokens.clone(), shell);
    if tokens != expand_abbr_tokens {
        delete_pre_buffer(*cursor);
        let new_target = tokens_to_string(&expand_abbr_tokens);
        *cursor = new_target.len();
        *buffer = new_target + tail;
        print_buffer_cursor(buffer, *cursor, None);
    }
}

fn history_suggestion(shell: &Arc<Mutex<Shell>>, buffer: &str) -> Option<String> {
    if buffer.is_empty() {
        return None;
    }
    let suggestion = {
        let sh = shell.lock().ok()?;
        sh.history.find_history_rev(buffer).cloned()?
    };
    if suggestion.len() <= buffer.len() {
        return None;
    }
    suggestion
        .strip_prefix(buffer)
        .filter(|s| !s.is_empty())
        .map(|rest| rest.to_string())
}
