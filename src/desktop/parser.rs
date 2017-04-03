use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::str;

use nom::{ErrorKind, IResult};
use nom::IResult::{Done, Error};

use super::model::*;
use super::error::*;

pub type ParseResult = Result<DesktopEntry, ParseError>;

const NON_UTF8: ErrorKind = ErrorKind::Custom(1);

/// Parse a slice of bytes into a `DesktopEntry`.
///
/// This parses a .desktop file (or similar) into a `DesktopEntry`.
/// If it is unable to successfully parse it returns an `Err`
pub fn parse<T: AsRef<[u8]>>(input: T) -> ParseResult {
    match desktop_entry(input.as_ref()) {
        Done(_, entry) => Ok(entry),
        Error(NON_UTF8) => Err(ParseError::NonUtf8),
        _ => Err(ParseError::Syntax),
    }
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

named!(desktop_entry<DesktopEntry>, dbg!(do_parse!(
        dbg!(blanks) >>
        groups: dbg!(many0!(group)) >>
        dbg!(eof!()) >>
        (DesktopEntry::new(groups)))));

named!(group<Group>, do_parse!(
        name: map_res!(header, str::from_utf8) >>
        char!('\n') >>
        values: key_value_list >>
        blanks >>
        (Group::new(name.into(), values))));

named!(header, delimited!(char!('['), take_while!(is_header_char), char!(']')));

named!(space, eat_separator!(&b" \t"[..]));
named!(endline<char>, alt!(value!('\0', eof!()) | char!('\n')));
// If we ever support serialization, we need a way to preserve comments
named!(comment, delimited!(char!('#'), is_not!(&b"\n"[..]), endline));
named!(empty_line, do_parse!(sp: space >> char!('\n') >> (sp)));
named!(blanks<()>, fold_many0!(alt!(comment | empty_line), (), |_,_| ()));

named!(key_value_list<HashMap<String, String>>, fold_many0!(
        entry,
        HashMap::new(),
        |mut acc: HashMap<String, String>, item: (String, String)| {
            acc.insert(item.0, item.1);
            acc
        }));

named!(entry<(String, String)>, do_parse!(
        blanks >>
        key:  entry_key >>
        space >>
        char!('=') >>
        space >>
        value: entry_value >>
        (key, value)));

const KEY_RE: &'static str = r"[A-Za-z0-9-]+(\[[a-z]{2}(_[A-Z]{2})?(.[A-Za-z0-9-]+)?(@[A-Za-z09-]+)?\])?";

named!(entry_key<String>, map!(re_bytes_find_static!(KEY_RE), |name| {
    use std::ascii::AsciiExt;
    // regex already garantees name is ascii
    let mut s = unsafe { str::from_utf8_unchecked(name).to_owned() };
    // name is case-insensitive, so lowercase it
    s.as_mut_str().make_ascii_lowercase();
    s
}));

fn entry_value(input: &[u8]) -> IResult<&[u8], String> {
    let (line, rest) = match input.iter().position(|&c| c == b'\n') {
        Some(p) => (&input[..p], &input[(p + 1)..]),
        None => (input, &[][..])
    };
    if let Ok(line) = str::from_utf8(line) {
        Done(rest, line.to_string())
    } else {
        Error(NON_UTF8)
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
        assert_eq!(entry_value(&[][..]), Done(&[][..], "".to_string()));
    }

    #[test]
    fn entry_value_test_basic() {
        assert_eq!(
            entry_value(&b"A simple value"[..]),
            Done(&b""[..], "A simple value".to_string()));
        assert_eq!(
            entry_value(&b"A simple value\n"[..]),
            Done(&b""[..], "A simple value".to_string()));
    }

    #[test]
    fn entry_value_test_escapes() {
        assert_eq!(
            entry_value(&b"\\s\\n\\t\\r\\\\\\a"[..]),
            Done(&b""[..], "\\s\\n\\t\\r\\\\\\a".to_string()));
        assert_eq!(
            entry_value(&b"Content with trailing slash \\"[..]),
            Done(&b""[..], "Content with trailing slash \\".to_string()))
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
            Done(&b""[..], "name[en_us.utf-8@shaw]".to_string()));
        assert_eq!(
            entry_key(&b"Name[en_US.UTF-8]"[..]),
            Done(&b""[..], "name[en_us.utf-8]".to_string()));
        assert_eq!(
            entry_key(&b"Name[en_US@shaw]"[..]),
            Done(&b""[..], "name[en_us@shaw]".to_string()));
        assert_eq!(
            entry_key(&b"Name[en.UTF-8@shaw]"[..]),
            Done(&b""[..], "name[en.utf-8@shaw]".to_string()));
        assert_eq!(
            entry_key(&b"Name[en_US]"[..]),
            Done(&b""[..], "name[en_us]".to_string()));
        assert_eq!(
            entry_key(&b"Name[en.UTF-8]"[..]),
            Done(&b""[..], "name[en.utf-8]".to_string()));
        assert_eq!(
            entry_key(&b"Name[en@shaw]"[..]),
            Done(&b""[..], "name[en@shaw]".to_string()));
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

        let expected = DesktopEntry::new(vec!(Group::new("Desktop Entry".into(), hash!{
            "value1".to_string() => "Some value".to_string(),
            "value2".to_string() => "true".to_string(),
            "value3".to_string() => "false".to_string(),
            "value4".to_string() => "5.6".to_string()
        })));

        assert_eq!(desktop_entry(bytes), Done(&b""[..], expected));
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

        let expected = DesktopEntry::new(vec!(
                Group::new("Desktop Entry".into(), hash!{
                    "exe".to_string() => "env A=a B=b sample-prog --foo --bar".to_string(),
                    "directory".to_string() => "/etc/foo".to_string(),
                    "enabled".to_string() => "true".to_string()
                }),
                Group::new("Sample".into(), hash!{
                    "comment".to_string() => "Stuff".to_string(),
                    "comment[en]".to_string() => "Stuff".to_string(),
                    "comment[de]".to_string() => "Zeug".to_string()
                })));
        assert_eq!(parse(input).unwrap(), expected);
    }
}
