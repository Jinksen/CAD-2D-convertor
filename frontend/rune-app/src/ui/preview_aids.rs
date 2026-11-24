//! Text and hint overlays of the 3D view: section plane label, axis triad letters and the
//! mesh note. The hint lives in a tooltip so it never truncates. Display only.

use rune::tok::{Theme, space, type_};
use rune::{El, text};

use crate::api::dto::SectionPlane;
use crate::render3d::triad;
use crate::state::AppState;

/// Triad geometry in points: arm length and the distance of its origin from the corner.
pub const TRIAD_LEN: f64 = 30.0;
pub const TRIAD_INSET: f32 = 40.0;

pub const HINT: &str = "Drag to orbit, right or middle drag to pan, wheel to zoom, click a body to select, F fit selection, Shift+F fit all";

/// "XY plane, z = 12.500 mm" in the display unit.
pub fn plane_label(s: &AppState) -> Option<String> {
    let imported = s.model.imported.as_ref()?;
    let f = &s.shell.number_format;
    Some(match s.section.request(&imported.bounds_mm)? {
        SectionPlane::XY { offset_mm } => format!("XY plane, z = {}", f.length(offset_mm)),
        SectionPlane::XZ { offset_mm } => format!("XZ plane, y = {}", f.length(offset_mm)),
        SectionPlane::YZ { offset_mm } => format!("YZ plane, x = {}", f.length(offset_mm)),
        SectionPlane::Custom { .. } => "Custom plane".to_string(),
    })
}

/// Coarse-mesh note, shown when the display mesh was decimated.
pub fn mesh_note(s: &AppState) -> Option<String> {
    let m = s.preview.mesh.as_ref().filter(|m| m.coarsened)?;
    Some(format!(
        "Coarse display mesh ({:.3} mm); sectioning uses exact geometry",
        m.deflection_mm
    ))
}

/// Overlay labels: plane (top left), mesh note (bottom left) and the X/Y/Z letters at the
/// ends of the triad (bottom left corner, following the camera).
pub fn labels(s: &AppState, t: &Theme) -> Vec<El> {
    let small = |msg: String| text(msg).font(type_::LABEL).color(t.text_secondary).inert();
    let mut out = vec![];
    if let Some(p) = plane_label(s) {
        out.push(
            small(p)
                .key("plane-label")
                .pin(Some(space::S2), Some(space::S1), None, None),
        );
    }
    let (_, h) = s.preview.viewport;
    if h > 0.0 {
        let origin = [TRIAD_INSET, h - TRIAD_INSET];
        for (name, d) in ["X", "Y", "Z"]
            .into_iter()
            .zip(triad(&s.preview.camera, TRIAD_LEN + 8.0))
        {
            out.push(
                small(name.to_string())
                    .key(format!("triad-{}", name.to_lowercase()))
                    .absolute(origin[0] + d[0] - 3.0, origin[1] + d[1] - 7.0),
            );
        }
    }
    if let Some(n) = mesh_note(s) {
        out.push(
            small(n)
                .key("mesh-note")
                .pin(Some(space::S2), None, None, Some(space::S1)),
        );
    }
    out
}
