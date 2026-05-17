use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ParseError {
    /// Syntax error
    Syntax(String),
    /// Invalid UTF-8 sequence
    NonUtf8,
    /// IO error
    IO(io::Error),
}

// Mostly needed for tests
impl std::cmp::PartialEq for ParseError {
    fn eq(&self, other: &Self) -> bool {
        use self::ParseError::*;
        match (self, other) {
            (Syntax(ref e1), Syntax(e2)) => e1 == e2,
            (NonUtf8, NonUtf8) => true,
            _ => false,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::ParseError::*;
        match *self {
            Syntax(ref e) => write!(fmt, "Invalid syntax: {}", e),
            NonUtf8 => write!(fmt, "Invalid Ut8 byte sequence in input"),
            IO(ref err) => write!(fmt, "{}", err),
        }
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            ParseError::IO(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> Self {
        ParseError::IO(err)
    }
}
