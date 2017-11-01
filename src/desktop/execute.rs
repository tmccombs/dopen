use std::process::{Command};
use std::str;
use std::os::unix::process::CommandExt;

use regex::{self, Captures, Regex};

use super::model::DesktopEntry;
use super::entries::{Icon, Name};
use desktop::entries::Exec;

pub trait Executor {
    fn execute(self) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct ExecContext<'a> {
    /// The Desktop Entry that is being executed
    source: &'a DesktopEntry,
    /// The path (or uri) to the desktop file
    source_path: Option<String>,
    /// A list of files (or uris) to pass to the command
    args: &'a [String],
}

pub enum Error {
    NoCommand,
    IncompleteEscape,
    IncompleteQuote,
    MultipleFileArgs,
    ExecuteFailed
}

fn split_command<'a>(command: &'a str) -> CommandWords<'a> {
    CommandWords {
        inner: command.chars()
    }
}

struct CommandWords<'a> {
    inner: str::Chars<'a>
}

impl<'a> Iterator for CommandWords<'a> {
    type Item = Result<String, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        use self::Error::*;
        if self.inner.as_str().is_empty() {
            return None;
        }
        let mut result = String::with_capacity(self.inner.as_str().len());
        let mut escaping = false;
        let mut in_quotes = false;
        while let Some(c) = self.inner.next() {
            match c {
                '"' if !escaping => in_quotes = !in_quotes,
                '\\' if in_quotes => {
                    if escaping {
                        result.push('\\');
                    }
                    escaping = !escaping;
                }
                ' ' if !in_quotes => {
                    result.shrink_to_fit();
                    return Some(Ok(result));
                }
                _ => {
                    result.push(c);
                    escaping = false;
                }
            }
        }
        if escaping {
            Some(Err(IncompleteEscape))
        } else if in_quotes {
            Some(Err(IncompleteQuote))
        } else {
            result.shrink_to_fit();
            Some(Ok(result))
        }
    }
}

struct ReplaceFlags<'a>(&'a ExecContext<'a>);

impl<'a> regex::Replacer for ReplaceFlags<'a> {
    fn replace_append(&mut self, cap: &Captures, dst: &mut String) {
        // FIXME? should we localize icon and name?
        match &cap[0] {
            // FIXME: this is actually supposed to use seperate commands for each
            // argument
            "%f" | "%u" => if let Some(f) = self.0.args.first() {
                dst.push_str(f);
            },
            "%i" => if let Some(Icon(i)) = self.0.source.get::<Icon>() {
                dst.push_str(&i);
            },
            "%c" => if let Some(Name(n)) = self.0.source.get::<Name>() {
                dst.push_str(&n);
            },
            "%k" => if let Some(ref p) = self.0.source_path {
                dst.push_str(p);
            },
            "%%" => dst.push('%'),
            _ => {} // unrecognized flag
        }
    }
}

pub fn parse_command<'a>(command: &str, context: &ExecContext<'a>) -> Result<Command, Error> {
    use self::Error::*;

    lazy_static! {
        static ref FLAG_RE: Regex = Regex::new("%.").unwrap();
    }

    let mut words = split_command(command);
    let bin = words.next().unwrap_or(Err(NoCommand))?;
    let mut command = Command::new(&bin);
    let mut had_file_or_url = false;
    for arg in words {
        let arg = arg?;
        if arg == "%F" || arg == "%U" {
            if had_file_or_url {
                return Err(MultipleFileArgs)
            }
            command.args(context.args);
            had_file_or_url = true;
        } else {
            let replaced = FLAG_RE.replace_all(&arg, ReplaceFlags(context));
            command.arg(replaced.as_ref());
        }
    }
    Ok(command)
}

pub struct CommandExecutor<'a> {
    entry: &'a DesktopEntry,
    command: Command
}

impl<'a> CommandExecutor<'a> {
    pub fn new(entry: &'a DesktopEntry, args: &'a [String], path: Option<String>) -> Result<CommandExecutor<'a>, Error> {
        let exec_str = entry.get::<Exec>().ok_or(Error::NoCommand)?;
        let command = parse_command(&exec_str, &ExecContext {
            source: entry,
            source_path: path,
            args
        })?;
        Ok(CommandExecutor {
            entry,
            command
        })
    }
}

impl<'a> Executor for CommandExecutor<'a> {
    fn execute(mut self) -> Result<(), Error> {
        // TODO: setup environment
        self.command.exec();
        Err(Error::ExecuteFailed)
    }
}

pub fn execute(entry: &DesktopEntry, args: &[String], path: Option<String>) -> Result<(), Error> {
    CommandExecutor::new(entry, args, path).and_then(Executor::execute)
}
