// src/error.rs
use std::{env, fmt, io};

/// このクレート全体で使う Result エイリアス
pub type Result<T> = std::result::Result<T, Error>;

/// まとめて扱うエラー型（必要に応じて後で増やせます）
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Fmt(fmt::Error),
    VarError(env::VarError),
    // 例: Utf8(std::str::Utf8Error),
    // 例: ParseInt(std::num::ParseIntError),
    // 例: Custom(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {e}"),
            Error::Fmt(e) => write!(f, "Format error: {e}"),
            Error::VarError(e) => write!(f, "Variable error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<fmt::Error> for Error {
    fn from(e: fmt::Error) -> Self {
        Error::Fmt(e)
    }
}

impl From<env::VarError> for Error {
    fn from(e: env::VarError) -> Self {
        Error::VarError(e)
    }
}
