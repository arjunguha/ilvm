use std::io;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    Usage(String),
    Parse(String),
    Runtime(String),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IO(error)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::IO(e) => e.fmt(f),
            Error::Usage(s) => f.write_str(s),
            Error::Parse(s) => f.write_str(s),
            Error::Runtime(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::IO(e) => e.description(),
            Error::Usage(s) => s,
            Error::Parse(s) => s,
            Error::Runtime(s) => s,
        }
    }
}
