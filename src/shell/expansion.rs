use std::{collections::BTreeMap, io::Write};

use super::exe_list::ExeList;

#[derive(Clone)]
pub struct Expansion {
    expansions: BTreeMap<String, String>,
    name: String,
}

pub type Aliases = Expansion;
pub type Abbrs = Expansion;

impl Expansion {
    pub fn new(name: String) -> Self {
        Self {
            expansions: BTreeMap::new(),
            name,
        }
    }
    pub fn insert(&mut self, key: String, val: String, exe_list: &mut ExeList) {
        self.expansions.insert(key.clone(), val);
        exe_list.insert(key);
    }
    pub fn get(&self, key: &str) -> Option<&String> {
        self.expansions.get(key)
    }
    pub fn display(&self, w: &mut dyn Write) {
        if self.expansions.is_empty() {
            let _ = writeln!(w, "No {} registered.", self.name);
        } else {
            let output = self
                .expansions
                .iter()
                .map(|(abbr, full)| format!("{} => {}", abbr, full))
                .collect::<Vec<_>>()
                .join("\n");
            let _ = writeln!(w, "{}", output);
        }
    }
}
