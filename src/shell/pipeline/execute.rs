// execute.rs（抜粋）
// 依存: std + libc（nix等は未使用）

use std::{
    fs::{File, OpenOptions},
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
};

use crate::{
    error::{Error, Result},
    shell::{
        Shell,
        builtins::{Io, find},
        // ▼ ここを parse から新しい型をインポート
        pipeline::parse::{CommandExpr, Expr, Redirection, Segment, WordNode},
    },
};
use libc::{dup, pipe};

// ────────────────────────────────────────────────────────────────────────────
// WordNode → String 変換（セグメント結合; 展開は pre_execute 済みを前提）
// ※ まだ pre_execute を入れていない段階でも、とりあえずセグメントを素結合します。
fn materialize(w: &WordNode) -> String {
    let mut s = String::new();
    for seg in &w.segments {
        match seg {
            Segment::Unquoted(t) | Segment::DoubleQuoted(t) | Segment::SingleQuoted(t) => {
                s.push_str(t)
            }
        }
    }
    s
}
// ────────────────────────────────────────────────────────────────────────────

// --- 低レベルFDヘルパ -------------------------------------------------------

const STDIN_FILENO: RawFd = 0;
const STDOUT_FILENO: RawFd = 1;
const STDERR_FILENO: RawFd = 2;

/// dup() -> OwnedFd
fn dup_fd(fd: RawFd) -> io::Result<OwnedFd> {
    let newfd = unsafe { dup(fd) };
    if newfd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(newfd) })
    }
}

/// pipe() -> (read, write) as OwnedFd
fn make_pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0; 2];
    let rc = unsafe { pipe(fds.as_mut_ptr()) };
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
    }
}

/// 継承FD(標準入出力)を File として使いたいときは dup() して所有権を取る
fn file_for_inherit(fd_num: RawFd) -> io::Result<File> {
    Ok(File::from(dup_fd(fd_num)?))
}
/// 外部コマンドへ渡すための Stdio を継承FDから作る（dup 不要: inheritでOK）
fn stdio_inherit() -> Stdio {
    Stdio::inherit()
}
/// OwnedFd -> Stdio
fn stdio_from_owned(fd: OwnedFd) -> Stdio {
    Stdio::from(File::from(fd))
}

/// リダイレクト用ファイルオープン
fn open_redirect_file(path: &str, append: bool) -> io::Result<File> {
    if append {
        OpenOptions::new().create(true).append(true).open(path)
    } else {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
    }
}

// --- 終了コード正規化 & wait_all -------------------------------------------

fn exit_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    if let Some(c) = status.code() {
        c
    } else if let Some(sig) = status.signal() {
        128 + sig
    } else {
        1
    }
}

fn wait_all(children: &mut Vec<Child>) -> Result<Vec<i32>> {
    children
        .drain(..)
        .map(|mut c| Ok(exit_code(c.wait()?)))
        .collect()
}

// --- エントリ ---------------------------------------------------------------

pub fn execute(expr: &Expr, shell: &Arc<Mutex<Shell>>) -> Result<i32> {
    match expr {
        Expr::And(lhs, rhs) => {
            if execute(lhs, shell)? == 0 {
                execute(rhs, shell)
            } else {
                Ok(1)
            }
        }
        Expr::Or(lhs, rhs) => {
            if execute(lhs, shell)? != 0 {
                execute(rhs, shell)
            } else {
                Ok(1)
            }
        }
        Expr::Pipe(commands) => execute_pipeline(commands, shell),
    }
}

// --- 中核: パイプライン実行 -------------------------------------------------

fn execute_pipeline(commands: &[CommandExpr], shell: &Arc<Mutex<Shell>>) -> Result<i32> {
    if commands.is_empty() {
        return Ok(0);
    }

    let mut children: Vec<Child> = Vec::new();
    let mut prev_read: Option<OwnedFd> = None;

    for (i, cmd) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;

        // ▼ WordNode → String（ここで確定）
        let cmd_name_str = materialize(&cmd.cmd_name);
        let args_str: Vec<String> = cmd.args.iter().map(materialize).collect();

        // 次ステージへのパイプが必要か？
        let need_next_pipe = !is_last
            && (matches!(cmd.stdout, Redirection::Pipe) || matches!(cmd.stderr, Redirection::Pipe));

        let (mut next_r, mut next_w) = (None::<OwnedFd>, None::<OwnedFd>);
        if need_next_pipe {
            let (r, w) = make_pipe()?;
            next_r = Some(r);
            next_w = Some(w);
        }

        // ===== ビルトインか？ =====
        if let Some(bi) = find(&cmd_name_str) {
            // stdin
            let mut in_file: File = if let Some(rfd) = prev_read.take() {
                File::from(rfd)
            } else {
                file_for_inherit(STDIN_FILENO)?
            };

            // stdout
            let mut out_file: File = match &cmd.stdout {
                Redirection::Pipe if next_w.is_some() => {
                    File::from(dup_fd(next_w.as_ref().unwrap().as_raw_fd())?)
                }
                Redirection::Inherit => file_for_inherit(STDOUT_FILENO)?,
                Redirection::File { path, append } => {
                    let p = materialize(path);
                    open_redirect_file(&p, *append)?
                }
                Redirection::Pipe => file_for_inherit(STDOUT_FILENO)?,
            };

            // stderr
            let mut err_file: File = match &cmd.stderr {
                Redirection::Pipe if next_w.is_some() => {
                    File::from(dup_fd(next_w.as_ref().unwrap().as_raw_fd())?)
                }
                Redirection::Inherit => file_for_inherit(STDERR_FILENO)?,
                Redirection::File { path, append } => {
                    let p = materialize(path);
                    open_redirect_file(&p, *append)?
                }
                Redirection::Pipe => file_for_inherit(STDERR_FILENO)?,
            };

            // 実行
            let mut io = Io {
                stdin: &mut in_file,
                stdout: &mut out_file,
                stderr: &mut err_file,
            };
            bi.run(shell, &args_str, &mut io);

            // パイプ下流へ EOF
            drop(out_file);
            drop(err_file);

            prev_read = next_r.take();
            continue;
        }

        // ===== 外部コマンド =====
        let mut c = Command::new(&cmd_name_str);
        c.args(&args_str);

        // stdin
        match prev_read.take() {
            Some(rfd) => {
                c.stdin(stdio_from_owned(rfd));
            }
            None => {
                c.stdin(stdio_inherit());
            }
        }

        // stdout
        match &cmd.stdout {
            Redirection::Pipe if next_w.is_some() => {
                let w_dup = dup_fd(next_w.as_ref().unwrap().as_raw_fd())?;
                c.stdout(Stdio::from(File::from(w_dup)));
            }
            Redirection::Inherit => {
                c.stdout(stdio_inherit());
            }
            Redirection::File { path, append } => {
                let p = materialize(path);
                let f = open_redirect_file(&p, *append)?;
                c.stdout(Stdio::from(f));
            }
            Redirection::Pipe => {
                c.stdout(stdio_inherit());
            }
        }

        // stderr
        match &cmd.stderr {
            Redirection::Pipe if next_w.is_some() => {
                let w_dup = dup_fd(next_w.as_ref().unwrap().as_raw_fd())?;
                c.stderr(Stdio::from(File::from(w_dup)));
            }
            Redirection::Inherit => {
                c.stderr(stdio_inherit());
            }
            Redirection::File { path, append } => {
                let p = materialize(path);
                let f = open_redirect_file(&p, *append)?;
                c.stderr(Stdio::from(f));
            }
            Redirection::Pipe => {
                c.stderr(stdio_inherit());
            }
        }

        // spawn
        let child = match c.spawn() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to start '{}': {}", cmd_name_str, e);
                return Err(Error::NoChild);
            }
        };

        // 親は書き端 next_w を保持不要（dup を子へ渡したら閉じる）
        drop(next_w.take());

        // 次段へ read 端を渡す
        prev_read = next_r.take();

        children.push(child);
    }

    let codes = wait_all(&mut children)?;
    Ok(*codes.last().unwrap_or(&0))
}
