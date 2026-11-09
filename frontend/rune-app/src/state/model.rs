//! Imported model: bodies, selection, display visibility and user groups. None of it reaches
//! sectioning or export; groups only organize the tree and are saved in the project.

use std::collections::BTreeSet;

use rune::TextEdit;
use serde::{Deserialize, Serialize};

use crate::api::dto::{ImportComponent, ImportSummary};
use crate::state::tree::{self, Folder};

/// A named set of bodies, such as "Magnets" or "Stator laminations". A body can be in several.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyGroup {
    pub name: String,
    pub component_ids: Vec<String>,
}

#[derive(Default)]
pub struct ModelState {
    pub imported: Option<ImportSummary>,
    pub selected: Option<String>,
    pub related: Option<String>,
    pub hidden: BTreeSet<String>,
    /// Bound to the tree's filter field.
    pub filter: TextEdit,
    pub groups: Vec<BodyGroup>,
    /// Name of the group shown in the property pane; exclusive with `selected`.
    pub selected_group: Option<String>,
    /// Collapsed assembly folders (`Folder::key`) and groups (`group_key`).
    pub collapsed: BTreeSet<String>,
    /// Name fields of "New group" and of the selected group's rename.
    pub new_group: TextEdit,
    pub rename_group: TextEdit,
}

/// Collapse key of a group; the prefix keeps it apart from assembly paths.
pub fn group_key(name: &str) -> String {
    format!("group:{name}")
}

/// Case-insensitive match on display name, export name or any hierarchy segment.
pub fn body_matches(component: &ImportComponent, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    needle.is_empty()
        || [&component.display_name, &component.export_name]
            .into_iter()
            .chain(&component.hierarchy_path)
            .any(|field| field.to_lowercase().contains(&needle))
}

impl ModelState {
    pub fn set_imported(&mut self, imported: ImportSummary) {
        *self = ModelState {
            imported: Some(imported),
            ..ModelState::default()
        };
    }

    pub fn import_id(&self) -> Option<&str> {
        self.imported.as_ref().map(|i| i.import_id.as_str())
    }

    pub fn select(&mut self, id: Option<String>) {
        self.selected = id;
        self.related = None;
        self.selected_group = None;
    }

    pub fn select_group(&mut self, name: &str) {
        self.select(None);
        self.selected_group = Some(name.to_owned());
        self.rename_group = TextEdit::new(name);
    }

    pub fn toggle_hidden(&mut self, id: &str) {
        if !self.hidden.remove(id) {
            self.hidden.insert(id.to_owned());
        }
    }

    /// Shows only `id`; isolating again restores all bodies.
    pub fn toggle_isolate(&mut self, id: &str) {
        self.isolate(&[id.to_owned()]);
    }

    /// Shows only `ids`; isolating the same set again restores all bodies.
    fn isolate(&mut self, ids: &[String]) {
        let others: BTreeSet<String> = self
            .imported
            .iter()
            .flat_map(|i| &i.components)
            .filter(|c| !ids.contains(&c.id))
            .map(|c| c.id.clone())
            .collect();
        let isolated = !others.is_empty() && others == self.hidden;
        self.hidden = if isolated { BTreeSet::new() } else { others };
    }

    pub fn toggle_collapsed(&mut self, key: &str) {
        if !self.collapsed.remove(key) {
            self.collapsed.insert(key.to_owned());
        }
    }

    /// While filtering, every match stays visible, so folders are shown expanded.
    pub fn is_collapsed(&self, key: &str) -> bool {
        self.filter.text.trim().is_empty() && self.collapsed.contains(key)
    }

    pub fn group(&self, name: &str) -> Option<&BodyGroup> {
        self.groups.iter().find(|g| g.name == name)
    }

    fn check_group_name(&self, name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a group name.".into());
        }
        if self
            .groups
            .iter()
            .any(|g| g.name.eq_ignore_ascii_case(name))
        {
            return Err(format!("A group named {name} already exists."));
        }
        Ok(name.to_owned())
    }

    /// Creates a group from the "New group" field, holding the selected body if there is one,
    /// and selects it.
    pub fn create_group(&mut self) -> Result<(), String> {
        let name = self.check_group_name(&self.new_group.text)?;
        let component_ids = self.selected.iter().cloned().collect();
        self.groups.push(BodyGroup {
            name: name.clone(),
            component_ids,
        });
        self.new_group.clear();
        self.select_group(&name);
        Ok(())
    }

    /// Renames the selected group to the rename field's text.
    pub fn rename_selected_group(&mut self) -> Result<(), String> {
        let Some(old) = self.selected_group.clone() else {
            return Ok(());
        };
        if self.rename_group.text.trim() == old {
            return Ok(());
        }
        let name = self.check_group_name(&self.rename_group.text)?;
        if let Some(group) = self.groups.iter_mut().find(|g| g.name == old) {
            group.name = name.clone();
        }
        if self.collapsed.remove(&group_key(&old)) {
            self.collapsed.insert(group_key(&name));
        }
        self.select_group(&name);
        Ok(())
    }

    pub fn delete_group(&mut self, name: &str) {
        self.groups.retain(|g| g.name != name);
        self.collapsed.remove(&group_key(name));
        if self.selected_group.as_deref() == Some(name) {
            self.selected_group = None;
        }
    }

    /// Adds `id` to the group, or removes it when it is already a member.
    pub fn toggle_member(&mut self, group: &str, id: &str) {
        if let Some(group) = self.groups.iter_mut().find(|g| g.name == group) {
            match group.component_ids.iter().position(|m| m == id) {
                Some(index) => {
                    group.component_ids.remove(index);
                }
                None => group.component_ids.push(id.to_owned()),
            }
        }
    }

    /// A group with members is hidden when all of them are.
    pub fn group_hidden(&self, name: &str) -> bool {
        self.group(name).is_some_and(|g| {
            !g.component_ids.is_empty() && g.component_ids.iter().all(|id| self.hidden.contains(id))
        })
    }

    /// Hides every member, or shows them all when the group is already hidden.
    pub fn toggle_group_hidden(&mut self, name: &str) {
        let hidden = self.group_hidden(name);
        let Some(group) = self.group(name) else {
            return;
        };
        for id in group.component_ids.clone() {
            if hidden {
                self.hidden.remove(&id);
            } else {
                self.hidden.insert(id);
            }
        }
    }

    pub fn toggle_group_isolate(&mut self, name: &str) {
        if let Some(group) = self.group(name) {
            let ids = group.component_ids.clone();
            self.isolate(&ids);
        }
    }

    /// Restores saved groups, dropping members the current STEP no longer has.
    pub fn restore_groups(&mut self, groups: Vec<BodyGroup>) {
        let known: BTreeSet<&str> = self
            .imported
            .iter()
            .flat_map(|i| &i.components)
            .map(|c| c.id.as_str())
            .collect();
        self.groups = groups
            .into_iter()
            .map(|mut g| {
                g.component_ids.retain(|id| known.contains(id.as_str()));
                g
            })
            .collect();
    }

    /// Components matching the tree filter.
    pub fn visible_bodies(&self) -> Vec<&ImportComponent> {
        let query = &self.filter.text;
        self.imported
            .iter()
            .flat_map(|i| &i.components)
            .filter(|c| body_matches(c, query))
            .collect()
    }

    /// Matching bodies nested in their assembly folders, without the levels all bodies share.
    pub fn visible_tree(&self) -> Folder<'_> {
        tree::skip_shared(tree::build(self.visible_bodies()))
    }

    /// Members of a group that match the tree filter, in model order.
    pub fn visible_members(&self, group: &BodyGroup) -> Vec<&ImportComponent> {
        self.visible_bodies()
            .into_iter()
            .filter(|c| group.component_ids.contains(&c.id))
            .collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::api::dto::BoundsMm;

    pub fn body(id: &str, name: &str, path: &[&str]) -> ImportComponent {
        ImportComponent {
            id: id.into(),
            source_id: None,
            source_name: Some(name.into()),
            display_name: name.into(),
            export_name: format!("{name}_1"),
            hierarchy_path: path.iter().map(|s| s.to_string()).collect(),
            source_color: Some([255, 0, 0]),
            name_provenance: None,
            color_provenance: None,
            body_ordinal: 0,
            bounds_mm: BoundsMm {
                min_xyz: [0.0; 3],
                max_xyz: [10.0, 20.0, 30.0],
            },
        }
    }

    pub fn summary(ids: &[&str]) -> ImportSummary {
        ImportSummary {
            import_id: "imp".into(),
            path: "m.step".into(),
            sha256: "x".into(),
            source_unit: Some("mm".into()),
            to_mm_scale: Some(1.0),
            bounds_mm: BoundsMm {
                min_xyz: [0.0; 3],
                max_xyz: [10.0, 20.0, 30.0],
            },
            component_count: ids.len(),
            components: ids.iter().map(|id| body(id, id, &[])).collect(),
            diagnostics: vec![],
        }
    }

    #[test]
    fn filter_matches_name_export_name_or_parent_and_blank_matches_all() {
        let rotor = body("b", "Rotor", &["Motor", "Rotor"]);
        assert!(body_matches(&rotor, "   "));
        assert!(body_matches(&rotor, "ROT"));
        assert!(body_matches(&rotor, "rotor_1"));
        assert!(body_matches(&rotor, "motor"));
        assert!(!body_matches(&rotor, "stator"));
    }

    #[test]
    fn filtered_bodies_keep_their_assembly_folders_and_ignore_collapse() {
        let mut m = ModelState::default();
        let mut s = summary(&[]);
        s.components = vec![
            body("1", "Rotor", &["Motor", "Rotor"]),
            body("2", "Stator", &["Motor", "Housing", "Stator"]),
        ];
        m.set_imported(s);
        let tree = m.visible_tree();
        assert_eq!((tree.key.as_str(), tree.bodies.len()), ("Motor", 1));
        m.toggle_collapsed("Motor / Housing");
        assert!(m.is_collapsed("Motor / Housing"));
        m.filter.text = "stat".into();
        assert!(!m.is_collapsed("Motor / Housing"));
        let tree = m.visible_tree();
        assert_eq!(
            (tree.key.as_str(), tree.bodies.len()),
            ("Motor / Housing", 1)
        );
        m.set_imported(summary(&["a", "b"]));
        let tree = m.visible_tree();
        assert!(tree.folders.is_empty() && tree.bodies.len() == 2);
    }

    #[test]
    fn groups_are_created_renamed_toggled_and_deleted() {
        let mut m = ModelState::default();
        m.set_imported(summary(&["a", "b", "c"]));
        assert!(m.create_group().unwrap_err().contains("name"));
        m.select(Some("a".into()));
        m.new_group = TextEdit::new(" Magnets ");
        m.create_group().unwrap();
        assert_eq!(m.selected_group.as_deref(), Some("Magnets"));
        assert!(m.selected.is_none());
        assert_eq!(m.group("Magnets").unwrap().component_ids, ["a"]);
        m.new_group = TextEdit::new("magnets");
        assert!(m.create_group().unwrap_err().contains("already exists"));

        m.toggle_member("Magnets", "b");
        m.toggle_group_hidden("Magnets");
        assert!(m.group_hidden("Magnets"));
        assert_eq!(m.hidden, ["a".to_string(), "b".to_string()].into());
        m.toggle_group_hidden("Magnets");
        assert!(m.hidden.is_empty());
        m.toggle_group_isolate("Magnets");
        assert_eq!(m.hidden, ["c".to_string()].into());
        m.toggle_member("Magnets", "b");
        assert_eq!(m.group("Magnets").unwrap().component_ids, ["a"]);

        m.toggle_collapsed(&group_key("Magnets"));
        m.rename_group = TextEdit::new("PM");
        m.rename_selected_group().unwrap();
        assert!(m.group("PM").is_some() && m.collapsed.contains(&group_key("PM")));
        m.delete_group("PM");
        assert!(m.groups.is_empty() && m.selected_group.is_none() && m.collapsed.is_empty());
    }

    #[test]
    fn restored_groups_drop_bodies_the_step_no_longer_has() {
        let mut m = ModelState::default();
        m.set_imported(summary(&["a"]));
        m.restore_groups(vec![BodyGroup {
            name: "G".into(),
            component_ids: vec!["a".into(), "gone".into()],
        }]);
        assert_eq!(m.groups[0].component_ids, ["a"]);
    }

    #[test]
    fn isolate_hides_the_others_and_a_second_isolate_restores_all() {
        let mut m = ModelState::default();
        m.set_imported(summary(&["a", "b", "c"]));
        m.toggle_isolate("b");
        assert_eq!(m.hidden, ["a".to_string(), "c".to_string()].into());
        m.toggle_isolate("b");
        assert!(m.hidden.is_empty());
    }

    #[test]
    fn hidden_toggles_and_a_new_import_resets_selection_and_visibility() {
        let mut m = ModelState::default();
        m.set_imported(summary(&["a", "b"]));
        m.toggle_hidden("a");
        m.select(Some("b".into()));
        assert!(m.hidden.contains("a") && m.selected.as_deref() == Some("b"));
        m.toggle_hidden("a");
        assert!(m.hidden.is_empty());
        m.toggle_hidden("a");
        m.set_imported(summary(&["a"]));
        assert!(m.hidden.is_empty() && m.selected.is_none());
    }
}
