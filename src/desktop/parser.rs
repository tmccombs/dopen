use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::str;

use winnow::ascii::{space0, till_line_ending};
use winnow::combinator::{alt, delimited, opt, preceded, repeat, separated_pair, terminated};
use winnow::stream::{AsChar, Stream};
use winnow::token::take_while;
use winnow::{Parser, Result};

use super::error::*;
use super::model::*;

pub type ParseResult = Result<DesktopEntry, ParseError>;

/// Parse a slice of bytes into a `DesktopEntry`.
///
/// This parses a .desktop file (or similar) into a `DesktopEntry`.
/// If it is unable to successfully parse it returns an `Err`
pub fn parse(input: &[u8]) -> ParseResult {
    desktop_entry
        .parse(input)
        .map_err(|e| ParseError::Syntax(e.to_string()))
}

pub fn parse_io<T: io::Read>(input: &mut T) -> ParseResult {
    let mut buf = Vec::new();
    input.read_to_end(&mut buf)?;
    parse(buf.as_ref())
}

pub fn parse_file<T: AsRef<Path>>(path: T) -> ParseResult {
    parse_io(&mut File::open(path)?)
}

fn desktop_entry(input: &mut &[u8]) -> Result<DesktopEntry> {
    preceded(blanks, repeat(0.., group).map(DesktopEntry::new)).parse_next(input)
}

fn group(i: &mut &[u8]) -> Result<Group> {
    let mut header = delimited(b'[', take_while(0.., is_header_char), b']').try_map(str::from_utf8);

    let name = header.parse_next(i)?;
    let values = delimited(b'\n', key_value_list, blanks).parse_next(i)?;
    Ok(Group::new(name.into(), values))
}

// If we ever support serialization, we need a way to preserve comments
fn comment<'s>(i: &mut &'s [u8]) -> Result<&'s [u8]> {
    preceded('#', till_line_ending).parse_next(i)
}
fn blanks(i: &mut &[u8]) -> Result<()> {
    let empty_line = terminated(space0, '\n');
    repeat(0.., alt((empty_line, comment))).parse_next(i)
}

fn key_value_list(i: &mut &[u8]) -> Result<HashMap<String, String>> {
    repeat(0.., entry).parse_next(i)
}

fn entry(i: &mut &[u8]) -> Result<(String, String)> {
    separated_pair(
        preceded(blanks, entry_key),
        delimited(space0, b'=', space0),
        entry_value,
    )
    .parse_next(i)
}

fn entry_key(i: &mut &[u8]) -> Result<String> {
    let identifier = || take_while(1.., (b'-', AsChar::is_alphanum));

    let locale = (
        take_while(2, b'a'..=b'z'),
        opt(preceded(b'_', take_while(2, b'A'..=b'Z'))),
        opt(preceded(b'.', identifier())),
        opt(preceded(b'@', identifier())),
    );
    let bytes = (identifier(), opt(delimited(b'[', locale, b']')))
        .take()
        .parse_next(i)?;

    // SAFETY: the parser should already guarantee the value is ascii
    let key = unsafe { str::from_utf8_unchecked(bytes) }.to_owned();
    Ok(key)
}

fn entry_value(i: &mut &[u8]) -> Result<String> {
    // Technically, it should be until "\n", but till_line_ending also looks for
    // "\r\n", and if that happens, the value should technically include the "\r", but
    // in practice I don't think that is an issue, and is more likely an issue with
    // the desktop file using the wrong line endings.
    let line = till_line_ending.try_map(str::from_utf8).parse_next(i)?;
    let _ = i.next_token();
    Ok(line.to_string())
}

fn is_header_char(c: u8) -> bool {
    // any ASCII char that isn't a control acharacter
    // or a square bracket
    (32u8..127u8).contains(&c) && c != b'[' && c != b']'
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! hash {
        ($($k:expr => $v:expr),*) => ({
            use std::collections::HashMap;
            let mut h = HashMap::new();
            $( h.insert($k, $v); )*
            h
        })
    }

    #[test]
    fn entry_value_test_empty() {
        assert_eq!(
            entry_value.parse_peek(&[][..]),
            Ok((&[][..], "".to_string()))
        );
    }

    #[test]
    fn entry_value_test_basic() {
        assert_eq!(
            entry_value.parse_peek(&b"A simple value\n"[..]),
            Ok((&b""[..], "A simple value".to_string()))
        );
        assert_eq!(
            entry_value.parse_peek(&b"A simple value"[..]),
            Ok((&b""[..], "A simple value".to_string()))
        );
    }

    #[test]
    fn entry_value_test_escapes() {
        assert_eq!(
            entry_value.parse_peek(&b"\\s\\n\\t\\r\\\\\\a"[..]),
            Ok((&b""[..], "\\s\\n\\t\\r\\\\\\a".to_string()))
        );
        assert_eq!(
            entry_value.parse_peek(&b"Content with trailing slash \\"[..]),
            Ok((&b""[..], "Content with trailing slash \\".to_string()))
        )
    }

    #[test]
    fn entry_value_test_invalid_utf8() {
        assert!(entry_value.parse_peek(&[0xc0, 0xc1]).is_err());
        assert!(entry_value.parse_peek(&[0x80, 0xc1]).is_err());
    }

    #[test]
    fn entry_key_test_locales() {
        assert_eq!(
            entry_key.parse_peek(&b"Name[en_US.UTF-8@shaw]"[..]),
            Ok((&b""[..], "Name[en_US.UTF-8@shaw]".to_string()))
        );
        assert_eq!(
            entry_key.parse_peek(&b"Name[en_US.UTF-8]"[..]),
            Ok((&b""[..], "Name[en_US.UTF-8]".to_string()))
        );
        assert_eq!(
            entry_key.parse_peek(&b"Name[en_US@shaw]"[..]),
            Ok((&b""[..], "Name[en_US@shaw]".to_string()))
        );
        assert_eq!(
            entry_key.parse_peek(&b"Name[en.UTF-8@shaw]"[..]),
            Ok((&b""[..], "Name[en.UTF-8@shaw]".to_string()))
        );
        assert_eq!(
            entry_key.parse_peek(&b"Name[en_US]"[..]),
            Ok((&b""[..], "Name[en_US]".to_string()))
        );
        assert_eq!(
            entry_key.parse_peek(&b"Name[en.UTF-8]"[..]),
            Ok((&b""[..], "Name[en.UTF-8]".to_string()))
        );
        assert_eq!(
            entry_key.parse_peek(&b"Name[en@shaw]"[..]),
            Ok((&b""[..], "Name[en@shaw]".to_string()))
        );
    }

    #[test]
    fn entry_test() {
        let bytes = &b"\
[Desktop Entry]
# A Comment
Value1=Some value
# Boolean values
Value2=true
Value3=false

# Floating point
Value4=5.6"[..];

        let expected = DesktopEntry::new(vec![Group::new(
            "Desktop Entry".into(),
            hash! {
                "Value1".to_string() => "Some value".to_string(),
                "Value2".to_string() => "true".to_string(),
                "Value3".to_string() => "false".to_string(),
                "Value4".to_string() => "5.6".to_string()
            },
        )]);

        assert_eq!(desktop_entry.parse_peek(bytes), Ok((&b""[..], expected)));
    }

    #[test]
    fn parse_test() {
        let input = b"\
[Desktop Entry]
#A comment
Exec=env A=a B=b sample-prog --foo --bar
Directory = /etc/foo
# A boolean value
Enabled=true

[Sample]
Comment=Stuff
Comment[en]=Stuff
Comment[de]=Zeug";

        let expected = DesktopEntry::new(vec![
            Group::new(
                "Desktop Entry".into(),
                hash! {
                    "Exec".to_string() => "env A=a B=b sample-prog --foo --bar".to_string(),
                    "Directory".to_string() => "/etc/foo".to_string(),
                    "Enabled".to_string() => "true".to_string()
                },
            ),
            Group::new(
                "Sample".into(),
                hash! {
                    "Comment".to_string() => "Stuff".to_string(),
                    "Comment[en]".to_string() => "Stuff".to_string(),
                    "Comment[de]".to_string() => "Zeug".to_string()
                },
            ),
        ]);
        assert_eq!(parse(input).unwrap(), expected);
    }
}
