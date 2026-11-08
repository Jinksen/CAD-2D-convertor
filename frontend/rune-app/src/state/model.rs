//! Imported model: bodies, selection, display visibility (never reaches sectioning or export).

use std::collections::BTreeSet;

use rune::TextEdit;

use crate::api::dto::{ImportComponent, ImportSummary};

#[derive(Default)]
pub struct ModelState {
    pub imported: Option<ImportSummary>,
    pub selected: Option<String>,
    pub related: Option<String>,
    pub hidden: BTreeSet<String>,
    /// Bound to the tree's filter field.
    pub filter: TextEdit,
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
    }

    pub fn toggle_hidden(&mut self, id: &str) {
        if !self.hidden.remove(id) {
            self.hidden.insert(id.to_owned());
        }
    }

    /// Shows only `id`; isolating again restores all bodies.
    pub fn toggle_isolate(&mut self, id: &str) {
        let others: BTreeSet<String> = self
            .imported
            .iter()
            .flat_map(|i| &i.components)
            .filter(|c| c.id != id)
            .map(|c| c.id.clone())
            .collect();
        let isolated = !others.is_empty() && others == self.hidden;
        self.hidden = if isolated { BTreeSet::new() } else { others };
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
