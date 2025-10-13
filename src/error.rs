use std::{env, fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Fmt(fmt::Error),
    VarError(env::VarError),
    NoChild,
    StructureCollaps,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {e}"),
            Error::Fmt(e) => write!(f, "Format error: {e}"),
            Error::VarError(e) => write!(f, "Variable error: {e}"),
            Error::NoChild => write!(f, "no child to wait on"),
            Error::StructureCollaps => write!(f, "Failed to parse tokens"),
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
