//! Section settings: plane, offset, run status and result statistics.

use rune::tok::{Theme, space, type_};
use rune::{Color, El, TextEdit, col, edit_access, row, text, text_input};

use super::{button, heading, label_value, severity_color, toggle};
use crate::api::dto::Severity;
use crate::state::section::PlaneKind;
use crate::state::{AppState, now_ms};

const OFFSET_STEP_MM: f64 = 1.0;
const AXES: [&str; 3] = ["x", "y", "z"];

/// The axis a principal plane is offset along, as an ISO quantity symbol.
fn plane_axis(kind: PlaneKind) -> &'static str {
    match kind {
        PlaneKind::XY => "z",
        PlaneKind::XZ => "y",
        PlaneKind::YZ => "x",
        PlaneKind::Custom => "d",
    }
}

fn note(t: &Theme, msg: impl Into<String>) -> El {
    text(msg)
        .font(type_::LABEL)
        .color(t.text_secondary)
        .w_full()
}

/// A labelled single-line number field; Enter applies it.
fn field(
    t: &Theme,
    key: &str,
    label: &str,
    edit: &TextEdit,
    placeholder: &str,
    bind: impl for<'a> Fn(&'a mut AppState) -> &'a mut TextEdit + 'static,
) -> El {
    row()
        .w_full()
        .gap(space::S2)
        .items_center()
        .child(
            text(label)
                .font(type_::LABEL)
                .color(t.text_secondary)
                .w(96.0),
        )
        .child(
            text_input(edit, type_::BODY)
                .key(key)
                .label(label)
                .single_line()
                .placeholder(placeholder, t.text_secondary)
                .on_edit(bind)
                .on_submit(|s: &mut AppState, _| s.section.commit_fields(now_ms()))
                .grow(1.0)
                .px(space::S1)
                .py(2.0)
                .border(1.0, t.separator),
        )
}

pub fn settings(s: &AppState, t: &Theme, accent: Color) -> El {
    let fmt = s.shell.number_format;
    s.section.fmt.set(fmt);
    let sec = &s.section;
    let kinds = [
        (PlaneKind::XY, "XY"),
        (PlaneKind::XZ, "XZ"),
        (PlaneKind::YZ, "YZ"),
        (PlaneKind::Custom, "Custom"),
    ];
    let plane_row = kinds
        .into_iter()
        .fold(row().gap(space::S1), |r, (kind, label)| {
            r.child(
                toggle(t, accent, &format!("plane-{label}"), label, sec.plane == kind)
                    .on_press(move |s: &mut AppState| s.section.set_plane(kind, now_ms())),
            )
        });
    let panel = col()
        .key("section-settings")
        .w_full()
        .child(heading(t, "Section"))
        .child(
            col()
                .w_full()
                .padding(space::S2)
                .gap(space::S2)
                .child(note(t, "Exact planar cut of all bodies"))
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
    let custom = sec.plane == PlaneKind::Custom;
    let offset = sec.effective_offset(&imported.bounds_mm);
    let unit = fmt.length_unit.symbol();
    let busy = s.shell.busy.is_some();

    let mut form = col().w_full().padding(space::S2).gap(space::S1);
    if custom {
        for (i, axis) in AXES.iter().enumerate() {
            form = form.child(field(
                t,
                &format!("normal-{axis}"),
                &format!("Normal n{axis}"),
                &sec.normal_text[i],
                "",
                edit_access(move |s: &mut AppState| &mut s.section.normal_text[i]),
            ));
        }
    }
    let label = format!("Offset {} ({unit})", plane_axis(sec.plane));
    form = form
        .child(field(
            t,
            "section-offset",
            &label,
            &sec.offset_text,
            &format!("{} (model midpoint)", fmt.length(offset)),
            edit_access(|s: &mut AppState| &mut s.section.offset_text),
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
    if custom {
        let selected = s.model.selected.is_some();
        form = form
            .child(
                button(t, "align-plane", "Align to selected body", !busy && selected)
                    .on_press(AppState::align_plane_to_selected),
            )
            .child(match sec.basis() {
                Some([n, ..]) => note(t, format!("Unit normal n = {}", fmt.direction(&n))),
                None => note(t, "The plane normal must not be zero."),
            })
            .child(note(
                t,
                "Custom plane: n . p = d. Select a sheet and align to cut it in its own plane.",
            ));
    } else {
        form = form.child(note(
            t,
            format!(
                "The plane is {} = const; the initial offset is the model midpoint.",
                plane_axis(sec.plane)
            ),
        ));
    }
    if let Some(error) = &sec.field_error {
        form = form.child(
            text(error.clone())
                .font(type_::LABEL)
                .color(severity_color(Severity::Error, t))
                .w_full(),
        );
    }
    let computing = s.shell.busy.as_deref() == Some("Computing exact section");
    form = form
        .child(
            button(
                t,
                "section-compute",
                if computing { "Computing..." } else { "Compute section" },
                !busy,
            )
            .on_press(AppState::compute_section),
        )
        .child(note(
            t,
            format!("History: {} undo, {} redo", sec.past.len(), sec.future.len()),
        ));

    // Run status and result.
    let mut result = col().w_full().padding(space::S2).gap(space::S1);
    result = match (&sec.last, sec.stats) {
        (None, _) => result.child(note(t, "No section computed.")),
        (Some(section), stats) => {
            let mut r = result;
            if let Some(st) = stats {
                r = r
                    .child(label_value(
                        t,
                        "Bodies",
                        format!("{} of {}", section.components.len(), imported.component_count),
                    ))
                    .child(label_value(t, "Regions", st.regions.to_string()))
                    .child(label_value(t, "Errors", st.errors.to_string()))
                    .child(label_value(t, "Warnings", st.warnings.to_string()));
            } else {
                r = r.child(
                    text("Stale: the plane changed since this section was computed.")
                        .font(type_::LABEL)
                        .color(severity_color(Severity::Warning, t))
                        .w_full(),
                );
            }
            r.child(match sec.export_blocker() {
                None => note(
                    t,
                    "Ready to export: draft DXF bundle; complex Maxwell import still needs verification.",
                ),
                Some(reason) => note(t, format!("Export unavailable. {reason}")),
            })
        }
    };
    panel.child(form).child(result)
}

#[cfg(test)]
mod tests {
    use rune::{Env, Runtime};

    use super::*;

    fn runtime(state: AppState) -> Runtime<AppState> {
        let mut rt = Runtime::new(
            state,
            |s: &AppState, cx| settings(s, &rune::tok::theme(true), cx.accent()),
            (320.0, 700.0),
            Env {
                system_dark: true,
                scale: 1.0,
            },
        );
        rt.frame(0.0);
        rt
    }

    fn loaded() -> AppState {
        let mut s = AppState::default();
        s.model
            .set_imported(crate::state::model::tests::summary(&["a"]));
        s
    }

    fn press(rt: &mut Runtime<AppState>, key: &str) {
        let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
    }

    #[test]
    fn the_custom_plane_exposes_normal_fields_and_the_other_planes_do_not() {
        let mut rt = runtime(loaded());
        assert!(rt.find("normal-x").is_none());
        press(&mut rt, "plane-Custom");
        assert_eq!(rt.state.section.plane, PlaneKind::Custom);
        assert!(rt.find("normal-z").is_some() && rt.find("align-plane").is_some());
    }

    #[test]
    fn offset_buttons_move_the_plane_and_write_the_field() {
        let mut rt = runtime(loaded());
        press(&mut rt, "offset-inc");
        assert_eq!(rt.state.section.offset_mm, Some(16.0));
        assert_eq!(rt.state.section.offset_text.text, "16.000");
        press(&mut rt, "plane-XZ");
        assert_eq!(rt.state.section.offset_text.text, "");
    }
}
