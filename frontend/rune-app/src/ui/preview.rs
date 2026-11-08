//! Viewport: 2D section drawing and 3D preview. Both are placeholders for now.

use rune::tok::{Theme, space, type_};
use rune::{El, col, text};

use super::heading;
use crate::state::AppState;
use crate::state::preview::ViewportMode;

pub fn viewport(s: &AppState, t: &Theme) -> El {
    let title = match s.preview.mode {
        ViewportMode::TwoD => "Section (2D)",
        ViewportMode::ThreeD => "Preview (3D)",
    };
    let body = match (&s.section.last, s.preview.mode) {
        (_, ViewportMode::ThreeD) => "3D preview is not implemented yet.".to_string(),
        (Some(section), ViewportMode::TwoD) => {
            let curves: usize = section
                .components
                .iter()
                .flat_map(|c| &c.wires)
                .map(|w| w.curves.len())
                .sum();
            format!(
                "2D drawing is not implemented yet. Section {} has {} bodies and {} curves.",
                section.section_id,
                section.components.len(),
                curves
            )
        }
        (None, ViewportMode::TwoD) if s.model.imported.is_some() => {
            "Compute a section to see it here.".to_string()
        }
        (None, ViewportMode::TwoD) => "Open or drop a STEP file.".to_string(),
    };
    col()
        .key("viewport")
        .role("region")
        .label("Viewport")
        .grow(1.0)
        .h_full()
        .min_w(0.0)
        .child(heading(t, title))
        .child(
            col().w_full().grow(1.0).center().child(
                text(body)
                    .font(type_::BODY)
                    .color(t.text_secondary)
                    .padding(space::S3),
            ),
        )
}
