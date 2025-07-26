use regex::Regex;
use std::env;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn get_prompt(width: usize) -> String {
    // ANSI Colors
    // let blue = "\x1b[34m";
    let cyan = "\x1b[36m";
    let yellow = "\x1b[33m";
    let green = "\x1b[32m";
    // let red = "\x1b[31m";
    let gray = "\x1b[90m"; // 薄いグレー
    let reset = "\x1b[0m";

    let user = get_username();
    let host = get_hostname();
    let cwd = get_current_dir();
    let clock = get_current_time_string();

    let git_info = match get_git_branch_and_status() {
        Some((branch, dirty)) => {
            if dirty {
                format!(" on {}{}{} x{}", green, branch, reset, reset)
            } else {
                format!(" on {}{}{}", green, branch, reset)
            }
        }
        None => String::new(),
    };

    let left = format!(
        "# {}{}@{}{}: {}{}{}{}{}",
        cyan, user, host, reset, yellow, cwd, reset, git_info, reset,
    );

    let space_count = width.saturating_sub(strip_ansi(&left).len() + 8); // clock は8文字 "hh:mm:ss"
    let spaces = " ".repeat(space_count);

    format!("{}{}{}{}{}", left, spaces, gray, clock, reset)
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

fn get_git_branch_and_status() -> Option<(String, bool)> {
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

    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;

    let dirty = !status_output.stdout.is_empty();

    Some((branch, dirty))
}

fn get_current_time_string() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let seconds = now.as_secs() % 86400;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

// ANSIコードを除去して文字数を数えるための関数
fn strip_ansi(s: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}
