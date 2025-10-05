use libc::read;
use std::{cmp::Ordering, io, os::fd::RawFd};

use crate::input::{
    key::{Key, parse_keys},
    stream::drain_burst,
};

pub mod key;
pub mod stream;

const STDIN_FD: RawFd = 0;

pub fn wait_keys(timeout_ms: i64) -> io::Result<Vec<Key>> {
    loop {
        let mut b: u8 = 0;
        let n = unsafe { read(STDIN_FD, &mut b as *mut u8 as *mut _, 1) };

        match n.cmp(&0) {
            Ordering::Less => {
                let err = io::Error::last_os_error();
                // シグナル割り込みはリトライ
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }
            Ordering::Equal => {
                // EOF（例: Ctrl-D が行頭で押されたとき等）
                return Ok(Vec::new());
            }
            Ordering::Greater => {}
        }

        let burst = drain_burst(b, timeout_ms)?;
        return Ok(parse_keys(&burst));
    }
}
