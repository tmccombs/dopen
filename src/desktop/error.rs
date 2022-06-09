use std::error;
use std::fmt;
use std::io;
use std::str::Utf8Error;

use nom::error::ErrorKind;

#[derive(Debug)]
pub enum ParseError {
    /// Syntax error
    Syntax(String, ErrorKind),
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
            (Syntax(ref s1, k1), Syntax(ref s2, k2)) => k1 == k2 && s1 == s2,
            (NonUtf8, NonUtf8) => true,
            _ => false,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::ParseError::*;
        match *self {
            Syntax(ref i, kind) => write!(fmt, "Invalid syntax [{:?}] at \"{}\"", kind, i),
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

impl nom::error::ParseError<&[u8]> for ParseError {
    fn from_error_kind(input: &[u8], kind: ErrorKind) -> Self {
        let input_sample = input[..std::cmp::min(input.len(), 16)]
            .escape_ascii()
            .to_string();
        ParseError::Syntax(input_sample, kind)
    }

    fn append(_i: &[u8], _kind: ErrorKind, other: Self) -> Self {
        other
    }
}

impl nom::error::FromExternalError<&[u8], Utf8Error> for ParseError {
    fn from_external_error(_: &[u8], _: ErrorKind, _: Utf8Error) -> Self {
        Self::NonUtf8
    }
}
