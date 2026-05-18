use std::os::unix::process::CommandExt;
use std::process::Command;
use std::str;

use super::entries::{Icon, Name, Path};
use super::model::DesktopEntry;
use crate::entries::Exec;

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

#[derive(Debug)]
pub enum Error {
    NoCommand,
    IncompleteEscape,
    IncompleteQuote,
    MultipleFileArgs,
    InvalidFieldCode(char),
    ExecuteFailed(std::io::Error),
}

fn split_command<'a>(command: &'a str) -> CommandWords<'a> {
    CommandWords {
        inner: command.chars(),
    }
}

struct CommandWords<'a> {
    inner: str::Chars<'a>,
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
        for c in self.inner.by_ref() {
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

// TODO: test for parse_command

pub fn build_command<'a>(command: &str, context: &ExecContext<'a>) -> Result<Command, Error> {
    use self::Error::*;

    eprintln!("command={}", command);
    let mut words = split_command(command);
    let bin = words.next().unwrap_or(Err(NoCommand))?;
    let mut command = Command::new(&bin);
    let mut had_file_or_url = false;
    'arg_loop: for arg in words {
        let arg = arg?;
        match arg.as_ref() {
            // %F, %U, and %i can only be used as arguments on their own.
            "%F" | "%U" => {
                if had_file_or_url {
                    return Err(MultipleFileArgs);
                }
                had_file_or_url = true;
                command.args(context.args);
            }
            // FIXME: should we localize the icon?
            "%i" => {
                if let Some(Icon(icon)) = context.source.get() {
                    command.arg("--icon");
                    command.arg(icon);
                }
            }
            s => {
                let mut remaining = s;
                let mut replaced = String::new();
                while let Some(idx) = remaining.find('%') {
                    // Add everything up to this point
                    replaced.push_str(&remaining[..idx]);
                    let code_idx = idx + 1;
                    let mut chars = remaining[code_idx..].chars();
                    let Some(code) = chars.next() else {
                        return Err(InvalidFieldCode('\0'));
                    };
                    match code {
                        // FIXME: this is actually supposed to use seperate commands for each
                        // argument
                        'f' | 'u' => {
                            if had_file_or_url {
                                return Err(MultipleFileArgs);
                            }
                            had_file_or_url = true;
                            if let Some(f) = context.args.first() {
                                replaced.push_str(f);
                            } else {
                                // If we don't have any files, then skip this argument
                                continue 'arg_loop;
                            }
                        }
                        // FIXME? should we localize the name
                        'c' => {
                            if let Some(Name(name)) = context.source.get() {
                                replaced.push_str(&name);
                            }
                        }
                        'k' => {
                            if let Some(path) = &context.source_path {
                                replaced.push_str(path);
                            }
                        }
                        '%' => {
                            replaced.push('%');
                        }
                        // Deprecated arguments should be ignored
                        'd' | 'D' | 'n' | 'N' | 'v' | 'm' => continue 'arg_loop,
                        _ => return Err(InvalidFieldCode(code)),
                    }
                    remaining = chars.as_str();
                }
                if replaced.is_empty() {
                    command.arg(s);
                } else {
                    command.arg(replaced);
                }
            }
        }
    }
    if let Some(Path(path)) = context.source.get() {
        command.current_dir(path);
    }
    Ok(command)
}

pub struct CommandExecutor<'a> {
    entry: &'a DesktopEntry,
    command: Command,
}

impl<'a> CommandExecutor<'a> {
    pub fn new(
        entry: &'a DesktopEntry,
        args: &'a [String],
        path: Option<String>,
    ) -> Result<CommandExecutor<'a>, Error> {
        let exec_str = entry.get::<Exec>().ok_or(Error::NoCommand)?;
        let command = build_command(
            &exec_str,
            &ExecContext {
                source: entry,
                source_path: path,
                args,
            },
        )?;
        Ok(CommandExecutor { entry, command })
    }

    pub fn entry(&self) -> &DesktopEntry {
        self.entry
    }
}

impl<'a> Executor for CommandExecutor<'a> {
    fn execute(mut self) -> Result<(), Error> {
        // TODO: setup environment
        let err = self.command.exec();
        Err(Error::ExecuteFailed(err))
    }
}

// TODO: dbus activation
pub fn execute(entry: &DesktopEntry, args: &[String], path: Option<String>) -> Result<(), Error> {
    CommandExecutor::new(entry, args, path).and_then(Executor::execute)
}
