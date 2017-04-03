use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ParseError {
    /// Syntax error
    Syntax,
    /// Invalid UTF-8 sequence
    NonUtf8,
    /// IO error
    IO(io::Error)
}

impl fmt::Display for ParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::ParseError::*;
        match *self {
            Syntax => write!(fmt, "Invalid syntax"),
            NonUtf8 => write!(fmt, "Invalid Ut8 byte sequence in input"),
            IO(ref err) => write!(fmt, "{}", err)
        }
    }
}

impl error::Error for ParseError {
    fn description(&self) -> &str {
        use self::ParseError::*;
        match *self {
            Syntax => "Invalid syntax",
            NonUtf8 => "Invalid Utf8 byte sequence",
            IO(ref err) => err.description()
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ParseError::IO(ref err) => Some(err),
            _ => None
        }
    }
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> ParseError {
        ParseError::IO(err)
    }
}


