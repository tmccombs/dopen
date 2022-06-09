use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::str;

use nom::{
    bytes::complete::take_while,
    character::complete::{char, space0},
    combinator::{all_consuming, eof, map, map_res, value},
    multi::{fold_many0, many0},
    sequence::{delimited, preceded, separated_pair, terminated},
    Finish, InputTakeAtPosition, Parser,
};
use nom_regex::bytes::re_find;
use once_cell::sync::OnceCell;
use regex::bytes::Regex;

use super::error::*;
use super::model::*;

pub type ParseResult = Result<DesktopEntry, ParseError>;

type IResult<'a, T> = nom::IResult<&'a [u8], T, ParseError>;

/// Parse a slice of bytes into a `DesktopEntry`.
///
/// This parses a .desktop file (or similar) into a `DesktopEntry`.
/// If it is unable to successfully parse it returns an `Err`
pub fn parse<T: AsRef<[u8]>>(input: T) -> ParseResult {
    all_consuming(desktop_entry)(input.as_ref())
        .finish()
        .map(|(_, e)| e)
}

pub fn parse_io<T: io::Read>(input: &mut T) -> ParseResult {
    let mut buf = Vec::new();
    input.read_to_end(&mut buf)?;
    println!("Read: {}", str::from_utf8(&buf).unwrap());
    parse(buf)
}

pub fn parse_file<T: AsRef<Path>>(path: T) -> ParseResult {
    println!("Path: {:?}", path.as_ref());
    parse_io(&mut File::open(path)?)
}

fn desktop_entry(input: &[u8]) -> IResult<DesktopEntry> {
    preceded(blanks, map(many0(group), DesktopEntry::new))(input)
}

fn group(i: &[u8]) -> IResult<Group> {
    let header = delimited(char('['), take_while(is_header_char), char(']'));

    let (i, name) = map_res(header, str::from_utf8)(i)?;
    let (i, values) = delimited(char('\n'), key_value_list, blanks)(i)?;
    Ok((i, Group::new(name.into(), values)))
}

// If we ever support serialization, we need a way to preserve comments
fn comment(i: &[u8]) -> IResult<&[u8]> {
    let endline = char('\n').or(value('\0', eof));
    delimited(char('#'), take_while(|c| c != b'\n'), endline)(i)
}
fn blanks(i: &[u8]) -> IResult<()> {
    let empty_line = terminated(space0, char('\n'));
    fold_many0(empty_line.or(comment), || (), |_, _| ())(i)
}

fn key_value_list(i: &[u8]) -> IResult<HashMap<String, String>> {
    fold_many0(
        entry,
        || HashMap::new(),
        |mut acc, item| {
            acc.insert(item.0, item.1);
            acc
        },
    )(i)
}

fn entry(i: &[u8]) -> IResult<(String, String)> {
    eprintln!("parsing entry: {}", str::from_utf8(i).unwrap_or(""));
    separated_pair(
        preceded(blanks, entry_key),
        delimited(space0, char('='), space0),
        entry_value,
    )(i)
}

const KEY_RE: &'static str =
    r"^[A-Za-z0-9-]+(\[[a-z]{2}(_[A-Z]{2})?(.[A-Za-z0-9-]+)?(@[A-Za-z09-]+)?\])?";

fn entry_key(i: &[u8]) -> IResult<String> {
    static RE_CELL: OnceCell<Regex> = OnceCell::new();
    let key_re = RE_CELL.get_or_init(|| Regex::new(KEY_RE).unwrap());

    re_find(key_re.clone())
        .map(|name| {
            // regex already garantees name is ascii
            let mut s = unsafe { str::from_utf8_unchecked(name) }.to_owned();
            eprintln!("got name: {}", s);
            // name is case-insensitive, so lowercase it
            s.as_mut_str().make_ascii_lowercase();
            s
        })
        .parse(i)
}

fn entry_value(i: &[u8]) -> IResult<String> {
    let (mut rest, line) = i.split_at_position_complete(|c| c == b'\n')?;
    if !rest.is_empty() {
        rest = &rest[1..];
    }
    match str::from_utf8(line) {
        //
        Ok(line) => Ok((rest, line.to_string())),
        Err(_) => Err(nom::Err::Failure(ParseError::NonUtf8)),
    }
}

fn is_header_char(c: u8) -> bool {
    // any ASCII char that isn't a control acharacter
    // or a square bracket
    c >= 32 && c < 127 && c != b'[' && c != b']'
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
        assert_eq!(entry_value(&[][..]), Ok((&[][..], "".to_string())));
    }

    #[test]
    fn entry_value_test_basic() {
        assert_eq!(
            entry_value(&b"A simple value"[..]),
            Ok((&b""[..], "A simple value".to_string()))
        );
        assert_eq!(
            entry_value(&b"A simple value\n"[..]),
            Ok((&b""[..], "A simple value".to_string()))
        );
    }

    #[test]
    fn entry_value_test_escapes() {
        assert_eq!(
            entry_value(&b"\\s\\n\\t\\r\\\\\\a"[..]),
            Ok((&b""[..], "\\s\\n\\t\\r\\\\\\a".to_string()))
        );
        assert_eq!(
            entry_value(&b"Content with trailing slash \\"[..]),
            Ok((&b""[..], "Content with trailing slash \\".to_string()))
        )
    }

    #[test]
    fn entry_value_test_invalid_utf8() {
        assert!(entry_value(&[0xc0, 0xc1]).is_err());
        assert!(entry_value(&[0x80, 0xc1]).is_err());
    }

    #[test]
    fn entry_key_test_locales() {
        assert_eq!(
            entry_key(&b"Name[en_US.UTF-8@shaw]"[..]),
            Ok((&b""[..], "name[en_us.utf-8@shaw]".to_string()))
        );
        assert_eq!(
            entry_key(&b"Name[en_US.UTF-8]"[..]),
            Ok((&b""[..], "name[en_us.utf-8]".to_string()))
        );
        assert_eq!(
            entry_key(&b"Name[en_US@shaw]"[..]),
            Ok((&b""[..], "name[en_us@shaw]".to_string()))
        );
        assert_eq!(
            entry_key(&b"Name[en.UTF-8@shaw]"[..]),
            Ok((&b""[..], "name[en.utf-8@shaw]".to_string()))
        );
        assert_eq!(
            entry_key(&b"Name[en_US]"[..]),
            Ok((&b""[..], "name[en_us]".to_string()))
        );
        assert_eq!(
            entry_key(&b"Name[en.UTF-8]"[..]),
            Ok((&b""[..], "name[en.utf-8]".to_string()))
        );
        assert_eq!(
            entry_key(&b"Name[en@shaw]"[..]),
            Ok((&b""[..], "name[en@shaw]".to_string()))
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
                "value1".to_string() => "Some value".to_string(),
                "value2".to_string() => "true".to_string(),
                "value3".to_string() => "false".to_string(),
                "value4".to_string() => "5.6".to_string()
            },
        )]);

        assert_eq!(desktop_entry(bytes), Ok((&b""[..], expected)));
    }

    #[test]
    fn parse_test() {
        let input = "\
[Desktop Entry]
#A comment
Exe=env A=a B=b sample-prog --foo --bar
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
                    "exe".to_string() => "env A=a B=b sample-prog --foo --bar".to_string(),
                    "directory".to_string() => "/etc/foo".to_string(),
                    "enabled".to_string() => "true".to_string()
                },
            ),
            Group::new(
                "Sample".into(),
                hash! {
                    "comment".to_string() => "Stuff".to_string(),
                    "comment[en]".to_string() => "Stuff".to_string(),
                    "comment[de]".to_string() => "Zeug".to_string()
                },
            ),
        ]);
        assert_eq!(parse(input).unwrap(), expected);
    }
}
