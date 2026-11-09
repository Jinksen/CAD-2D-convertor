//! Assembly folders of the model tree, built from each body's STEP hierarchy path.

use crate::api::dto::ImportComponent;

/// One assembly level. `key` is the joined parent path, stable across sessions of the same STEP.
#[derive(Debug, Default)]
pub struct Folder<'a> {
    pub key: String,
    pub name: String,
    pub folders: Vec<Folder<'a>>,
    pub bodies: Vec<&'a ImportComponent>,
}

impl Folder<'_> {
    pub fn count(&self) -> usize {
        self.bodies.len() + self.folders.iter().map(Folder::count).sum::<usize>()
    }
}

/// Nests bodies under their parent assemblies, keeping model order at every level.
pub fn build<'a>(bodies: impl IntoIterator<Item = &'a ImportComponent>) -> Folder<'a> {
    let mut root = Folder::default();
    for body in bodies {
        let path = &body.hierarchy_path;
        let parents = &path[..path.len().saturating_sub(1)];
        let mut folder = &mut root;
        for depth in 0..parents.len() {
            let key = parents[..=depth].join(" / ");
            let index = match folder.folders.iter().position(|f| f.key == key) {
                Some(index) => index,
                None => {
                    folder.folders.push(Folder {
                        key,
                        name: parents[depth].clone(),
                        ..Folder::default()
                    });
                    folder.folders.len() - 1
                }
            };
            folder = &mut folder.folders[index];
        }
        folder.bodies.push(body);
    }
    root
}

/// The tree drops levels every body shares (usually the STEP's root assembly), so the
/// first visible level is the first one that tells bodies apart.
pub fn skip_shared(mut root: Folder<'_>) -> Folder<'_> {
    while root.bodies.is_empty() && root.folders.len() == 1 {
        root = root.folders.remove(0);
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::model::tests::body;

    #[test]
    fn nests_by_parent_path_and_skips_the_shared_root() {
        let bodies = [
            body("1", "Rotor", &["Motor", "Rotor"]),
            body("2", "Stator", &["Motor", "Housing", "Stator"]),
            body("3", "Cover", &["Motor", "Housing", "Cover"]),
            body("4", "Loose", &["Loose"]),
        ];
        let root = build(&bodies);
        assert_eq!(root.count(), 4);
        assert_eq!(root.bodies.len(), 1, "a top-level body sits in the root");
        let motor = &root.folders[0];
        assert_eq!((motor.key.as_str(), motor.count()), ("Motor", 3));
        let housing = &motor.folders[0];
        assert_eq!(housing.key, "Motor / Housing");
        assert_eq!(housing.bodies.len(), 2);

        let shared = skip_shared(build(&bodies[..3]));
        assert_eq!(shared.key, "Motor");
        assert_eq!(shared.bodies.len(), 1);
        assert_eq!(shared.folders[0].name, "Housing");
    }
}
