// execute.rs（抜粋）
// 依存: std + libc（nix等は未使用）

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::unix::process::CommandExt,
    process::{Child, ChildStdout, Command, ExitStatus, Stdio},
};

use crate::{
    error::{Error, Result},
    pipeline::parse::{CommandExpr, Expr, Redirection},
    shell::{Shell, builtins::find},
};

/// 外部コマンドへ渡すための Stdio を継承FDから作る（dup 不要: inheritでOK）
fn stdio_inherit() -> Stdio {
    Stdio::inherit()
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

pub fn execute(expr: &Expr, shell: &mut Shell) -> Result<i32> {
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

enum PipedOut {
    External(ChildStdout),
    Builtin(String),
    None,
}

impl PipedOut {
    #[inline]
    fn take(&mut self) -> Self {
        std::mem::replace(self, PipedOut::None)
    }
}

// --- 中核: パイプライン実行 -------------------------------------------------

fn execute_pipeline(commands: &[CommandExpr], shell: &mut Shell) -> Result<i32> {
    if commands.is_empty() {
        return Ok(0);
    }

    let mut children: Vec<Child> = Vec::new();
    let mut piped_out = PipedOut::None;

    for (i, cmd) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;

        // ▼ WordNode → String（ここで確定）
        let cmd_name_str = &cmd.cmd_name.concat_text();
        let args_str: Vec<String> = cmd.args.iter().map(|x| x.concat_text()).collect();
        let mut pending_stdin_from_builtin: Option<String> = None;

        // ===== ビルトインか？ =====
        if let Some(bi) = find(cmd_name_str) {
            wait_all(&mut children)?;
            let ret = bi.run(shell, &args_str);
            let mut piped = String::new();
            match &cmd.stdout {
                Redirection::File { path, append } => {
                    let p = path.concat_text();
                    let mut f = open_redirect_file(&p, *append)?;
                    f.write_all(ret.stdout.as_bytes())?;
                }
                Redirection::Inherit => {
                    print!("{}", ret.stdout);
                }
                Redirection::Pipe => {
                    piped += &ret.stdout;
                }
            }
            match &cmd.stderr {
                Redirection::File { path, append } => {
                    let p = path.concat_text();
                    let mut f = open_redirect_file(&p, *append)?;
                    f.write_all(ret.stderr.as_bytes())?;
                }
                Redirection::Inherit => {
                    eprint!("{}", ret.stderr);
                }
                Redirection::Pipe => {
                    piped += &ret.stderr;
                }
            }
            piped_out = PipedOut::Builtin(piped);
            continue;
        }

        // ===== 外部コマンド =====
        let mut c = Command::new(cmd_name_str);
        c.args(&args_str);
        let pre_piped_out = piped_out.take();

        // stdin 1
        match pre_piped_out {
            PipedOut::None => {
                c.stdin(Stdio::inherit());
            }
            PipedOut::Builtin(s) => {
                // ここでは pipe を開くだけ。実際の書き込みは spawn 後に行う
                c.stdin(Stdio::piped());
                pending_stdin_from_builtin = Some(s);
            }
            PipedOut::External(child_out) => {
                // ここで所有権を消費してそのまま子プロセスの stdin につなぐ
                c.stdin(Stdio::from(child_out));
            }
        };

        // stdout
        match &cmd.stdout {
            Redirection::Pipe => {
                c.stdout(Stdio::piped());
            }
            Redirection::Inherit => {
                c.stdout(stdio_inherit());
            }
            Redirection::File { path, append } => {
                let p = path.concat_text();
                let f = open_redirect_file(&p, *append)?;
                c.stdout(Stdio::from(f));
            }
        }

        // stderr
        match &cmd.stderr {
            Redirection::Pipe => {
                c.stderr(Stdio::piped());
            }
            Redirection::Inherit => {
                c.stderr(stdio_inherit());
            }
            Redirection::File { path, append } => {
                let p = path.concat_text();
                let f = open_redirect_file(&p, *append)?;
                c.stderr(Stdio::from(f));
            }
        }
        unsafe {
            c.pre_exec(|| {
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                libc::signal(libc::SIGTSTP, libc::SIG_DFL);
                libc::signal(libc::SIGTTIN, libc::SIG_DFL);
                libc::signal(libc::SIGTTOU, libc::SIG_DFL);
                Ok(())
            });
        }

        // spawn
        let mut child = match c.spawn() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to start '{}': {}", cmd_name_str, e);
                return Err(Error::NoChild);
            }
        };

        // stdin 2
        if let Some(s) = pending_stdin_from_builtin
            && let Some(mut stdin) = child.stdin.take()
        {
            use std::io::Write;
            stdin.write_all(s.as_bytes())?;
            drop(stdin);
        }
        if !is_last {
            piped_out = PipedOut::External(child.stdout.take().unwrap());
        }
        children.push(child);
    }

    let codes = wait_all(&mut children)?;
    Ok(*codes.last().unwrap_or(&0))
}
