//! Section settings: plane, offset, computed statistics.

use rune::tok::{Theme, space, type_};
use rune::{Color, El, col, row, text};

use super::{button, heading, label_value, toggle};
use crate::state::section::PlaneKind;
use crate::state::{AppState, now_ms};

const OFFSET_STEP_MM: f64 = 1.0;

pub fn settings(s: &AppState, t: &Theme, accent: Color) -> El {
    let fmt = s.shell.number_format;
    let planes = [
        (PlaneKind::XY, "XY"),
        (PlaneKind::XZ, "XZ"),
        (PlaneKind::YZ, "YZ"),
    ];
    let plane_row = planes
        .into_iter()
        .fold(row().gap(space::S1), |r, (kind, label)| {
            r.child(
                toggle(
                    t,
                    accent,
                    &format!("plane-{label}"),
                    label,
                    s.section.plane == kind,
                )
                .on_press(move |s: &mut AppState| s.section.set_plane(kind, now_ms())),
            )
        });
    let mut panel = col()
        .key("section-settings")
        .w_full()
        .child(heading(t, "Section"))
        .child(
            col()
                .w_full()
                .padding(space::S2)
                .gap(space::S2)
                .child(plane_row),
        );
    let Some(imported) = &s.model.imported else {
        return panel.child(
            text("No model imported.")
                .font(type_::BODY)
                .color(t.text_secondary)
                .padding(space::S2),
        );
    };
    let offset = s.section.effective_offset(&imported.bounds_mm);
    let source = if s.section.offset_mm.is_some() {
        ""
    } else {
        " (model midpoint)"
    };
    let stats = s.section.stats;
    let mut details = col()
        .w_full()
        .padding(space::S2)
        .gap(space::S1)
        .child(label_value(
            t,
            "Offset (mm)",
            format!("{}{source}", fmt.length(offset)),
        ))
        .child(
            row()
                .gap(space::S1)
                .child(
                    button(t, "offset-dec", "-1 mm", true).on_press(move |s: &mut AppState| {
                        s.section.set_offset(offset - OFFSET_STEP_MM, now_ms())
                    }),
                )
                .child(
                    button(t, "offset-inc", "+1 mm", true).on_press(move |s: &mut AppState| {
                        s.section.set_offset(offset + OFFSET_STEP_MM, now_ms())
                    }),
                ),
        );
    details = match stats {
        Some(st) => details
            .child(label_value(t, "Regions", st.regions.to_string()))
            .child(label_value(t, "Errors", st.errors.to_string()))
            .child(label_value(t, "Warnings", st.warnings.to_string())),
        None => details.child(
            text("Section not computed for this plane.")
                .font(type_::LABEL)
                .color(t.text_secondary),
        ),
    };
    panel = panel.child(details);
    panel
}
