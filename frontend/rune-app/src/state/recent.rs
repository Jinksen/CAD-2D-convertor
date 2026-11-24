//! The recent-projects list kept in the prefs file, newest first.

use std::path::{Path, PathBuf};

/// Hard cap of the stored list; the setting only limits how many are shown.
pub const MAX_STORED: usize = 50;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Recent {
    pub paths: Vec<PathBuf>,
}

impl Recent {
    pub fn from_strings(paths: Vec<String>) -> Recent {
        let mut r = Recent::default();
        for p in paths.into_iter().rev() {
            r.push(PathBuf::from(p));
        }
        r
    }

    pub fn to_strings(&self) -> Vec<String> {
        self.paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    /// Moves (or adds) `path` to the front.
    pub fn push(&mut self, path: PathBuf) {
        self.paths.retain(|p| p != &path);
        self.paths.insert(0, path);
        self.paths.truncate(MAX_STORED);
    }

    pub fn remove(&mut self, path: &Path) {
        self.paths.retain(|p| p != path);
    }

    /// The entries to show: the newest `count`.
    pub fn shown(&self, count: usize) -> &[PathBuf] {
        &self.paths[..count.min(self.paths.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_first_without_duplicates_and_limited_when_shown() {
        let mut r = Recent::default();
        for name in ["a", "b", "c", "a"] {
            r.push(PathBuf::from(name));
        }
        assert_eq!(r.to_strings(), ["a", "c", "b"]);
        assert_eq!(r.shown(2).len(), 2);
        r.remove(Path::new("c"));
        assert_eq!(r.to_strings(), ["a", "b"]);
        assert_eq!(
            Recent::from_strings(r.to_strings()),
            r,
            "round trip keeps the order"
        );
    }
}
