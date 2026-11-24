//! Application settings kept in the Rune prefs file, grouped like the settings dialog. Theme and
//! number format live in `ShellState` (their own prefs keys); everything else is here.
//! Defaults match the behaviour before the dialog existed.

use serde::{Deserialize, Serialize};

pub const RECENT_COUNTS: [usize; 4] = [5, 10, 15, 20];
pub const FILL_OPACITIES: [f32; 3] = [0.15, 0.28, 0.5];
pub const LINE_WIDTHS: [f32; 3] = [0.75, 1.0, 1.5];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Background3d {
    /// Follows the application theme.
    #[default]
    Theme,
    Dark,
    Light,
}

impl Background3d {
    pub const ALL: [Background3d; 3] = [Self::Theme, Self::Dark, Self::Light];

    /// RGB of the 3D background; `theme_bg` is the application background.
    pub fn rgb(self, theme_bg: [u8; 3]) -> [u8; 3] {
        match self {
            Self::Theme => theme_bg,
            Self::Dark => [30, 32, 36],
            Self::Light => [244, 245, 247],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Draft,
    #[default]
    Normal,
    Fine,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Self::Draft, Self::Normal, Self::Fine];

    /// Render resolution relative to the window's pixels.
    pub fn render_scale(self) -> f32 {
        match self {
            Self::Draft => 0.5,
            Self::Normal => 1.0,
            Self::Fine => 1.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MousePreset {
    #[default]
    Cad2Maxwell,
    Inventor,
    Ansys,
}

/// What a drag does in the 3D view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragAction {
    Orbit,
    Pan,
    /// A plain click picks a body; a drag does nothing.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
}

impl MousePreset {
    pub const ALL: [MousePreset; 3] = [Self::Cad2Maxwell, Self::Inventor, Self::Ansys];

    pub fn label(self) -> &'static str {
        match self {
            Self::Cad2Maxwell => "CAD2Maxwell",
            Self::Inventor => "Inventor",
            Self::Ansys => "ANSYS",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Cad2Maxwell => "Left drag orbits; middle or right drag pans. Wheel zooms.",
            Self::Inventor => {
                "Middle drag pans; Shift + middle drag orbits. Left click picks. Wheel zooms."
            }
            Self::Ansys => {
                "Middle drag orbits; Shift + middle or right drag pans. Left click picks. Wheel zooms."
            }
        }
    }

    pub fn action(self, button: Button, shift: bool) -> DragAction {
        use Button::{Left, Middle, Right};
        use DragAction::{None, Orbit, Pan};
        match (self, button, shift) {
            (Self::Cad2Maxwell, Left, _) => Orbit,
            (Self::Cad2Maxwell, _, _) => Pan,
            (Self::Inventor, Middle, false) => Pan,
            (Self::Inventor, Middle, true) => Orbit,
            (Self::Ansys, Middle, false) => Orbit,
            (Self::Ansys, Middle, true) | (Self::Ansys, Right, _) => Pan,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayerMode {
    #[default]
    Component,
    Group,
    Hybrid,
}

impl LayerMode {
    pub const ALL: [LayerMode; 3] = [Self::Component, Self::Group, Self::Hybrid];

    #[allow(dead_code)] // sent with the export request (next ticket)
    pub fn wire(self) -> &'static str {
        match self {
            Self::Component => "component",
            Self::Group => "group",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    #[default]
    Original,
    Group,
    Material,
    Monochrome,
}

impl ColorMode {
    pub const ALL: [ColorMode; 4] = [
        Self::Original,
        Self::Group,
        Self::Material,
        Self::Monochrome,
    ];

    #[allow(dead_code)] // sent with the export request (next ticket)
    pub fn wire(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Group => "group",
            Self::Material => "material",
            Self::Monochrome => "monochrome",
        }
    }
}

/// Export formats the service writes; DXF is always written.
pub const OPTIONAL_FORMATS: [&str; 5] = ["csv", "svg", "femm", "gmsh", "pyaedt"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub recent_count: usize,
    pub confirm_unsaved: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Viewport {
    pub fill_opacity: f32,
    pub line_width: f32,
    pub grid_default: bool,
    pub background_3d: Background3d,
    pub quality: Quality,
    pub mouse: MousePreset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Export {
    /// Folder the save dialog starts in; `None` uses the system default.
    pub folder: Option<String>,
    pub layer_mode: LayerMode,
    pub color_mode: ColorMode,
    /// Optional formats written next to the DXF files.
    pub formats: Vec<String>,
    /// Write one ZIP bundle; otherwise the files go into a folder named like the bundle.
    pub zip: bool,
}

impl Default for General {
    fn default() -> Self {
        General {
            recent_count: 10,
            confirm_unsaved: true,
        }
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            fill_opacity: 0.28,
            line_width: 1.0,
            grid_default: false,
            background_3d: Background3d::default(),
            quality: Quality::default(),
            mouse: MousePreset::default(),
        }
    }
}

impl Default for Export {
    fn default() -> Self {
        Export {
            folder: None,
            layer_mode: LayerMode::default(),
            color_mode: ColorMode::default(),
            formats: vec![],
            zip: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub general: General,
    pub viewport: Viewport,
    pub export: Export,
}

impl Settings {
    /// A saved value forced back into range (a hand-edited prefs file).
    pub fn sanitized(mut self) -> Settings {
        let near = |v: f32, set: &[f32]| set.iter().copied().find(|s| (s - v).abs() < 1e-3);
        self.general.recent_count = self.general.recent_count.clamp(1, 50);
        let v = &mut self.viewport;
        v.fill_opacity = near(v.fill_opacity, &FILL_OPACITIES).unwrap_or(0.28);
        v.line_width = near(v.line_width, &LINE_WIDTHS).unwrap_or(1.0);
        self.export
            .formats
            .retain(|f| OPTIONAL_FORMATS.contains(&f.as_str()));
        self.export.formats.dedup();
        self
    }
}

/// The dialog's categories, in list order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Category {
    #[default]
    General,
    Units,
    Viewport,
    Export,
    Service,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Self::General,
        Self::Units,
        Self::Viewport,
        Self::Export,
        Self::Service,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Units => "Units and numbers",
            Self::Viewport => "Viewport",
            Self::Export => "Export",
            Self::Service => "Service",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Units => "units",
            Self::Viewport => "viewport",
            Self::Export => "export",
            Self::Service => "service",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_maps_each_button_and_shift_to_one_action() {
        use Button::{Left, Middle, Right};
        use DragAction::{None, Orbit, Pan};
        let p = MousePreset::Cad2Maxwell;
        assert_eq!(
            (
                p.action(Left, false),
                p.action(Middle, false),
                p.action(Right, false)
            ),
            (Orbit, Pan, Pan)
        );
        let p = MousePreset::Inventor;
        assert_eq!(
            (
                p.action(Left, false),
                p.action(Middle, false),
                p.action(Middle, true)
            ),
            (None, Pan, Orbit)
        );
        let p = MousePreset::Ansys;
        assert_eq!(
            (
                p.action(Middle, false),
                p.action(Middle, true),
                p.action(Right, false)
            ),
            (Orbit, Pan, Pan)
        );
    }

    #[test]
    fn settings_default_to_the_behaviour_before_the_dialog() {
        let s = Settings::default();
        assert_eq!(s.viewport.fill_opacity, 0.28);
        assert_eq!(s.export.formats.len(), 0);
        assert!(s.export.zip && s.general.confirm_unsaved);
    }

    #[test]
    fn a_hand_edited_file_is_forced_back_into_range() {
        let mut s = Settings::default();
        s.general.recent_count = 999;
        s.viewport.fill_opacity = 7.0;
        s.export.formats = vec!["svg".into(), "bogus".into()];
        let s = s.sanitized();
        assert_eq!(s.general.recent_count, 50);
        assert_eq!(s.viewport.fill_opacity, 0.28);
        assert_eq!(s.export.formats, vec!["svg".to_string()]);
    }

    #[test]
    fn a_partial_prefs_file_fills_the_rest_with_defaults() {
        let s: Settings = serde_json::from_str(r#"{"viewport":{"grid_default":true}}"#).unwrap();
        assert!(s.viewport.grid_default);
        assert_eq!(s.viewport.line_width, 1.0);
        assert_eq!(s.general, General::default());
    }
}
