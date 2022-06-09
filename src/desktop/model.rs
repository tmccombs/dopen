use std::collections::HashMap;
use std::ops::Index;
use std::slice;

use super::entries::Entry;

pub const DESKTOP_ENTRY_NAME: &'static str = "Desktop Entry";

#[derive(Debug, PartialEq, Clone)]
pub struct Group {
    name: String,
    values: HashMap<String, String>,
}

impl Group {
    pub fn new(name: String, values: HashMap<String, String>) -> Group {
        Group {
            name: name,
            values: values,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn values(&self) -> &HashMap<String, String> {
        &self.values
    }

    pub fn get<T: Entry>(&self) -> Option<T> {
        self.get_raw(T::name()).and_then(T::deserialize)
    }

    // FIXME: This is overly simplistic, it needs to look up increasingly more general locales
    pub fn get_localized<T: Entry>(&self, locale: &str) -> Option<T> {
        self.get_raw(&format!("{}[{}]", T::name(), locale))
            .and_then(T::deserialize)
    }

    pub fn get_raw(&self, name: &str) -> Option<&str> {
        // name is case insensitive
        self.values
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DesktopEntry(Vec<Group>);

impl DesktopEntry {
    pub fn new(groups: Vec<Group>) -> DesktopEntry {
        DesktopEntry(groups)
    }

    /// Get a group in the entry by name
    pub fn group(&self, name: &str) -> Option<&Group> {
        self.0.iter().find(|g| g.name == name)
    }

    /// Get an iterator over all groups in the entry
    pub fn groups(&self) -> slice::Iter<Group> {
        self.0.iter()
    }

    /// Get the "Desktop Entry" group
    pub fn main_group(&self) -> Option<&Group> {
        self.group(DESKTOP_ENTRY_NAME)
    }

    pub fn action_group(&self, action_name: &str) -> Option<&Group> {
        self.group(&format!("Desktop Action {}", action_name))
    }

    /// Shortut for `self.main_group().get()`
    #[inline]
    pub fn get<T: Entry>(&self) -> Option<T> {
        self.main_group().and_then(Group::get)
    }
}

impl<'a> Index<&'a str> for DesktopEntry {
    type Output = Group;
    fn index(&self, group_name: &'a str) -> &Group {
        self.group(group_name).unwrap()
    }
}
