//! What an export sends: the export settings plus the model's groups with their material labels.

use crate::api::dto::{ExportGroup, ExportOptions};
use crate::state::AppState;

impl AppState {
    /// Export options from the settings and the groups. Empty groups are left out; DXF is
    /// always written and comes first.
    pub fn export_options(&self) -> ExportOptions {
        let e = &self.shell.settings.export;
        let mut formats = vec!["dxf".to_owned()];
        formats.extend(e.formats.iter().filter(|f| *f != "dxf").cloned());
        ExportOptions {
            layer_mode: e.layer_mode.wire().into(),
            color_mode: e.color_mode.wire().into(),
            groups: self
                .model
                .groups
                .iter()
                .filter(|g| !g.component_ids.is_empty())
                .map(|g| ExportGroup {
                    name: g.name.clone(),
                    members: g.component_ids.clone(),
                    material: g.material.clone(),
                    material_note: g
                        .material
                        .as_deref()
                        .filter(|m| *m == "custom")
                        .and(g.material_note.clone()),
                })
                .collect(),
            formats,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::state::AppState;
    use crate::state::model::BodyGroup;
    use crate::state::settings::{ColorMode, LayerMode};

    #[test]
    fn options_follow_the_settings_and_carry_group_materials() {
        let mut s = AppState::default();
        s.shell.edit_settings(|x| {
            x.export.layer_mode = LayerMode::Hybrid;
            x.export.color_mode = ColorMode::Material;
            x.export.formats = vec!["svg".into(), "csv".into()];
        });
        s.model.groups = vec![
            BodyGroup {
                name: "Magnets".into(),
                component_ids: vec!["a".into()],
                material: Some("permanent_magnet".into()),
                material_note: Some("dropped".into()),
            },
            BodyGroup {
                name: "Coil".into(),
                component_ids: vec!["b".into()],
                material: Some("custom".into()),
                material_note: Some("Litz".into()),
            },
            BodyGroup {
                name: "Empty".into(),
                ..BodyGroup::default()
            },
        ];
        let o = s.export_options();
        assert_eq!(
            (o.layer_mode.as_str(), o.color_mode.as_str()),
            ("hybrid", "material")
        );
        assert_eq!(o.formats, ["dxf", "svg", "csv"]);
        assert_eq!(o.groups.len(), 2, "empty groups are not sent");
        assert_eq!(
            o.groups[0].material_note, None,
            "a note only belongs to custom"
        );
        assert_eq!(o.groups[1].material_note.as_deref(), Some("Litz"));
    }
}
