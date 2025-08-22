use std::collections::BTreeMap;

pub(crate) struct Expansion {
    expansions: BTreeMap<String, String>,
    name: String,
}

pub(crate) type Aliases = Expansion;
pub(crate) type Abbrs = Expansion;

impl Expansion {
    pub(crate) fn new(name: String) -> Self {
        Self {
            expansions: BTreeMap::new(),
            name
        }
    }
    pub(crate) fn insert(&mut self, key: String, val: String, is_dirty: &mut bool) {
        *is_dirty = true;
        self.expansions.insert(key, val);
    }
    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.expansions.contains_key(key)
    }
    pub(crate) fn get(&self, key: &str) -> Option<&String> {
        self.expansions.get(key)
    }
    pub(crate) fn keys(&self) -> std::collections::btree_map::Keys<'_, String, String> {
        self.expansions.keys()
    }
    pub(crate) fn display(&self) {
        if self.expansions.is_empty() {
            println!("No {} registered.", self.name);
        } else {
            let output = self.expansions
                .iter()
                .map(|(abbr, full)| format!("{} => {}", abbr, full))
                .collect::<Vec<_>>()
                .join("\n");
            println!("{output}");
        }
    }
}
