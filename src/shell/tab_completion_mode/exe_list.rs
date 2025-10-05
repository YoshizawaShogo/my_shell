use std::collections::BTreeSet;

/// PATH上の実行可能ファイルが変更されることは稀だと考えるので、考慮しないこととする。
pub struct ExeList {
    executables: BTreeSet<String>,
    pre_path: String,
}

impl ExeList {
    pub fn new() -> Self {
        Self {
            executables: BTreeSet::new(),
            pre_path: String::new(),
        }
    }
    pub fn insert(&mut self, executable: String) {
        self.executables.insert(executable);
    }
}
