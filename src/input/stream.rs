use libc::{FD_SET, FD_ZERO, fd_set, read, select, timeval};
use std::cmp::Ordering;
use std::io;
use std::os::fd::RawFd;

const STDIN_FD: RawFd = 0;

/// 最初の1バイトを受け取った後、短いタイムアウトで連続バイトを吸い上げる
pub fn drain_burst(first: u8, timeout_ms: i64) -> io::Result<Vec<u8>> {
    let mut buf = vec![first];
    let fd = STDIN_FD;
    let nfds = fd + 1;
    let t_ms = timeout_ms.max(0);

    loop {
        let mut rfds = unsafe { std::mem::zeroed::<fd_set>() };
        unsafe {
            FD_ZERO(&mut rfds);
            FD_SET(fd, &mut rfds);
        }
        let mut tv = timeval {
            tv_sec: (t_ms / 1000) as _,
            tv_usec: ((t_ms % 1000) * 1000) as _,
        };

        // select 待ち（EINTR はリトライ）
        let ready = loop {
            let r = unsafe {
                select(
                    nfds,
                    &mut rfds,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut tv,
                )
            };
            if r < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue; // シグナル割り込みは再試行
                }
                return Err(err);
            }
            break r;
        };

        if ready == 0 {
            break; // タイムアウト
        }

        // 1バイト読み（EINTR はリトライ）
        let byte = loop {
            let mut b: u8 = 0;
            let n = unsafe { read(fd, &mut b as *mut u8 as *mut _, 1) };
            match n.cmp(&0) {
                Ordering::Less => {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err);
                }
                Ordering::Equal => {
                    // EOF
                    return Ok(buf);
                }
                Ordering::Greater => {
                    break b;
                }
            }
        };

        buf.push(byte);
    }

    Ok(buf)
}
