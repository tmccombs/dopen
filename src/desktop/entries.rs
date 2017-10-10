use std::ops::Deref;
use std::string;
use std::str::{FromStr, ParseBoolError};

/// Type representing a single entry in a group
pub trait Entry: FromStr {
    /// The name of the entry
    ///
    /// This the string of the
    #[inline(always)]
    fn name() -> &'static str;

    /// Deserialize an entry value from a string.
    #[inline]
    fn deserialize(v: &str) -> Option<Self> {
        v.parse().ok()
    }
}

macro_rules! entry_type {
    ($(#[$a:meta])* $name:ident (bool)) => {
        $(#[$a])*
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct $name(pub bool);
        impl Entry for $name {
            #[inline(always)]
            fn name() -> &'static str { stringify!($name) }
        }
        impl FromStr for $name {
            type Err = ParseBoolError;
            fn from_str(s: &str) -> Result<$name, ParseBoolError> {
                s.parse().map($name)
            }
        }
        impl Deref for $name {
            type Target = bool;
            fn deref(&self) -> &bool {
                &self.0
            }
        }
    };
    ($(#[$a:meta])* $name:ident(String)) => {
        $(#[$a])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(pub String);
        impl Entry for $name {
            #[inline(always)]
            fn name() -> &'static str { stringify!($name) }
        }
        impl FromStr for $name {
            type Err = string::ParseError;
            fn from_str(s: &str) -> Result<$name, string::ParseError> {
                Ok($name(util::unescape_value(s)))
            }
        }
        impl Deref for $name {
            type Target = str;
            fn deref(&self) -> &str {
                &self.0
            }
        }
        impl ToString for $name {
            fn to_string(&self) -> String {
                self.0.clone()
            }
        }
    };
    ($(#[$a:meta])* $name:ident(Vec<String>)) => {
        $(#[$a])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(pub Vec<String>);
        impl Entry for $name {
            #[inline(always)]
            fn name() -> &'static str { stringify!($name) }
        }
        impl FromStr for $name {
            type Err = string::ParseError;
            fn from_str(s: &str) -> Result<$name, string::ParseError> {
                Ok($name(util::split_value_str(s).collect()))
            }
        }
        impl Deref for $name {
            type Target = [String];
            fn deref(&self) -> &[String] {
                &self.0
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Application,
    Link,
    Directory,
    Unknown(String)
}
impl Entry for Type {
    #[inline(always)]
    fn name() -> &'static str { "Type" }
}
impl FromStr for Type {
    type Err = string::ParseError;

    fn from_str(s: &str) -> Result<Type, string::ParseError> {
        Ok(match s {
            "Application" => Type::Application,
            "Link" => Type::Link,
            "Directory" => Type::Directory,
            other => Type::Unknown(other.into())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Category {
    AudioVideo,
    Audio,
    Video,
    Development,
    Education,
    Game,
    Graphics,
    Network,
    Office,
    Science,
    Settings,
    System,
    Utility,
    Custom(String)
}
impl FromStr for Category {
    type Err = string::ParseError;
    fn from_str(s: &str) -> Result<Category, string::ParseError> {
        use self::Category::*;
        Ok(match s {
            "AudioVideo" => AudioVideo,
            "Audio" => Audio,
            "Video" => Video,
            "Development" => Development,
            "Education" => Education,
            "Game" => Game,
            "Graphics" => Graphics,
            "Network" => Network,
            "Office" => Office,
            "Science" => Science,
            "Settings" => Settings,
            "System" => System,
            "Utility" => Utility,
            other => Custom(other.into())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Categories(Vec<Category>);
impl Entry for Categories {
    #[inline(always)]
    fn name() -> &'static str { "Categories" }
}
impl FromStr for Categories {
    type Err = string::ParseError;
    fn from_str(s: &str) -> Result<Categories, string::ParseError> {
        Ok(Categories(
                util::split_value_str(s).map(|v| v.parse::<Category>().unwrap()).collect()))
    }
}
impl Deref for Categories {
    type Target = [Category];
    fn deref(&self) -> &[Category] {
        &self.0
    }
}

entry_type!(Version(String));
entry_type!(Name(String));
entry_type!(GenericName(String));
entry_type!(NoDisplay(bool));
entry_type!(Comment(String));
entry_type!(Icon(String));
entry_type!(Hidden(bool));
entry_type!(OnlyShowIn(Vec<String>));
entry_type!(NotShowIn(Vec<String>));
entry_type!(DBusActivatable(bool));
entry_type!(TryExec(String));
entry_type!(Exec(String));
entry_type!(Path(String));
entry_type!(Terminal(bool));
entry_type!(Actions(Vec<String>));
entry_type!(MimeType(Vec<String>));
entry_type!(Implements(Vec<String>));
entry_type!(Keywords(Vec<String>));
entry_type!(StartupNotify(bool));
entry_type!(StartupWMClass(String));
entry_type!(URL(String));

pub mod util {
    use std::str::Chars;

    /// Split a value by semicolons
    ///
    /// This split a string value into a `Vec` for multiple values
    /// as described in
    /// https://standards.freedesktop.org/desktop-entry-spec/latest/ar01s03.html
    pub fn split_value_str(s: &str) -> Values {
        Values {
            inner: s.chars()
        }
    }

    /// Unescape a string value
    ///
    /// This should be used when deserializing any entries
    /// with string values.
    pub fn unescape_value(s: &str) -> String {
        let mut content = String::with_capacity(s.len());
        let mut iter = s.chars();
        while let Some(ch) = iter.next() {
            if ch == '\\' {
                match iter.next() {
                    Some('s') => content.push(' '),
                    Some('n') => content.push('\n'),
                    Some('t') => content.push('\t'),
                    Some('r') => content.push('\r'),
                    Some('\\') | None => content.push('\\'),
                    Some(lit) => {
                        content.push('\\');
                        content.push(lit);
                    }
                }
            } else {
                content.push(ch);
            }
        }
        content.shrink_to_fit();
        content
    }

    /// Iterator over multiple string values in an entry.
    ///
    /// See `split_value_str`
    pub struct Values<'a> {
        inner: Chars<'a>
    }
    impl<'a> Iterator for Values<'a> {
        type Item = String;
        fn next(&mut self) -> Option<String> {
            if self.inner.as_str().is_empty() {
                return None;
            }
            let mut value = String::new();
            while let Some(c) = self.inner.next() {
                match c {
                    '\\' => match self.inner.next() {
                        Some('\\') | None => value.push('\\'),
                        Some(';') => value.push(';'),
                        Some('s') => value.push(' '),
                        Some('n') => value.push('\n'),
                        Some('t') => value.push('\t'),
                        Some('r') => value.push('\r'),
                        Some(lit) => {
                            value.push('\\');
                            value.push(lit);
                        }
                    },
                    ';' => return Some(value),
                    _ => value.push(c)
                };
            }
            value.shrink_to_fit();
            Some(value)
        }

    }
}


#[cfg(test)]
mod tests {
    use super::util::*;

    macro_rules! assert_strings_eq {
        ($expected:expr, [$($s:expr),*]) => {
            assert_eq!($expected.collect::<Vec<String>>(), vec![$($s.to_string()),*]);
        };
    }

    #[test]
    fn split_value_str_test() {
        assert_eq!(split_value_str("").next(), None);
        assert_strings_eq!(split_value_str(";"), [""]);
        assert_strings_eq!(split_value_str("a;b;c;d"), ["a", "b", "c", "d"]);
        assert_strings_eq!(split_value_str("a;b;"), ["a", "b"]);
        assert_strings_eq!(split_value_str("a;;"), ["a", ""]);
        assert_strings_eq!(split_value_str("a\\;b"), ["a;b"]);
        assert_strings_eq!(split_value_str("a\\;;b\\;"), ["a;", "b;"]);
        assert_strings_eq!(split_value_str("a\\b"), ["a\\b"]);
    }

    #[test]
    fn split_value_str_escape_test() {
        assert_strings_eq!(
            split_value_str("\\s\\n\\t\\r\\\\\\a\\;"),
            [" \n\t\r\\\\a;"]);
        assert_strings_eq!(split_value_str("a\\\\;b\\"), ["a\\", "b\\"]);
    }

    #[test]
    fn unescape_value_test() {
        assert_eq!(
            unescape_value("\\s\\n\\t\\r\\\\\\a\\;"),
            " \n\t\r\\\\a\\;".to_string());
        assert_eq!(unescape_value("a\\"), "a\\".to_string());
    }
}
