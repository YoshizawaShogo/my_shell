use std::{
    env, fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::output::{
    ansi::{
        color::{Color, fg},
        util::strip_ansi,
    },
    term_size::read_terminal_size,
};

pub fn get_prompt() -> String {
    let user = get_username();
    let host = get_hostname();
    let cwd = get_current_dir();
    let clock = get_current_time_string();

    let git_info = match get_git_branch() {
        Some(branch) => format!(
            " on {green}{branch}{reset}",
            green = fg(Color::Green),
            reset = fg(Color::Reset),
        ),
        None => String::new(),
    };

    let left = format!(
        "# {cyan}{user}@{host}{reset}: {yellow}{cwd}{reset}{git_info}{reset}",
        cyan = fg(Color::Cyan),
        yellow = fg(Color::Yellow),
        reset = fg(Color::Reset),
    );

    let width: usize = read_terminal_size().width.into();
    let space_count = width.saturating_sub(strip_ansi(&left).len() + 8); // "hh:mm:ss" -> 8文字
    let spaces = " ".repeat(space_count);

    format!(
        "{left}{spaces}{gray}{clock}{reset}\r\n",
        gray = fg(Color::BrightBlack),
        reset = fg(Color::Reset)
    )
}

fn get_username() -> String {
    env::var("USER").unwrap_or_else(|_| "unknown".to_string())
}

fn get_hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string()
}

fn get_current_dir() -> String {
    let path = env::current_dir().unwrap_or_default();
    let home = env::var("HOME").unwrap_or_default();

    if let Ok(stripped) = path.strip_prefix(&home) {
        format!("~/{}", stripped.display())
    } else {
        path.display().to_string()
    }
}

fn get_git_branch() -> Option<String> {
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if !branch_output.status.success() {
        return None;
    }

    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    Some(branch)
}

fn get_current_time_string() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    // JSTはUTC+9時間（9*3600秒）
    let seconds = (now.as_secs() + 9 * 3600) % 86400;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
