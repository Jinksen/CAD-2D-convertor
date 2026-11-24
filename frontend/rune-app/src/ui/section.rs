//! Section settings: plane, offset, run status and result statistics.

use rune::tok::{Theme, space, type_};
use rune::{El, TextEdit, col, edit_access, row, text};
use rune_kit::{Kit, TextInput};

use super::{button, heading, label_value, severity_color};
use crate::api::dto::Severity;
use crate::state::section::{PlaneKind, tolerance};
use crate::state::{AppState, now_ms};

const OFFSET_STEP_MM: f64 = 1.0;
const AXES: [&str; 3] = ["x", "y", "z"];

/// The axis a principal plane is offset along, as an ISO quantity symbol.
pub(super) fn plane_axis(kind: PlaneKind) -> &'static str {
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
        .wrap()
        .w_full()
}

/// A labelled single-line number field; Enter applies it.
fn field(
    t: &Theme,
    k: &Kit,
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
            TextInput::new(edit)
                .label(label)
                .placeholder(placeholder)
                .width(None)
                .height(22.0)
                .on_edit(bind)
                .on_submit(|s: &mut AppState, _| s.section.commit_fields(now_ms()))
                .build(k)
                .key(key)
                .grow(1.0),
        )
}

/// "Advanced tolerances": the three validation overrides, collapsed by default. Bounds are
/// shown, an invalid entry is rejected inline and a non-default value is marked.
fn tolerances(s: &AppState, t: &Theme, k: &Kit) -> El {
    let tol = &s.section.tolerance;
    let open = tol.open;
    let title = format!(
        "Advanced tolerances{}",
        if tol.is_default() { "" } else { " (modified)" }
    );
    let mut out = col()
        .key("advanced-tolerances")
        .w_full()
        .gap(space::S1)
        .child(
            button(k, "toggle-tolerances", &title, true)
                .checked(open)
                .on_press(|s: &mut AppState| s.section.tolerance.open = !s.section.tolerance.open),
        );
    if !open {
        return out;
    }
    for (i, f) in tolerance::FIELDS.iter().enumerate() {
        let range = format!(
            "Default {}, range {} to {} {}",
            tolerance::show(f.default),
            tolerance::show(f.min),
            tolerance::show(f.max),
            f.unit
        );
        let label = format!("{}{}", f.label, if tol.is_set(i) { " *" } else { "" });
        out = out
            .child(field(
                t,
                k,
                f.key,
                &label,
                &tol.text[i],
                &tolerance::show(f.default),
                edit_access(move |s: &mut AppState| &mut s.section.tolerance.text[i]),
            ))
            .child(note(t, range));
        if let Some(error) = &tol.error[i] {
            out = out.child(
                text(error.clone())
                    .font(type_::LABEL)
                    .color(severity_color(Severity::Error, t))
                    .wrap()
                    .w_full(),
            );
        }
    }
    out.child(note(
        t,
        "Empty fields use the defaults. * marks a value that differs from them; it is saved in the project and applies at the next Compute.",
    ))
}

/// Named sections of the project: switch in the tree, add, rename and delete here.
fn named_sections(s: &AppState, t: &Theme, k: &Kit) -> El {
    let named = &s.section.named;
    let many = named.list.len() > 1;
    let mut out = col()
        .key("named-sections")
        .w_full()
        .gap(space::S1)
        .child(note(
            t,
            format!(
                "Named sections: {} (active: {}). Switch in the model tree.",
                named.list.len(),
                s.section.active_name().unwrap_or("Section 1")
            ),
        ))
        .child(field(
            t,
            k,
            "section-name",
            "Section name",
            &named.name_edit,
            "Section name",
            edit_access(|s: &mut AppState| &mut s.section.named.name_edit),
        ))
        .child(
            row()
                .gap(space::S1)
                .child(button(k, "section-add", "Add section", true).on_press(
                    |s: &mut AppState| {
                        let name = s.section.named.name_edit.text.clone();
                        s.section.named.error = s.section.add_named(&name).err();
                    },
                ))
                .child(
                    button(k, "section-rename", "Rename", true).on_press(|s: &mut AppState| {
                        let name = s.section.named.name_edit.text.clone();
                        s.section.named.error = s.section.rename_active(&name).err();
                    }),
                )
                .child(
                    button(k, "section-delete", "Delete", many).on_press(|s: &mut AppState| {
                        let active = s.section.named.active;
                        s.section.named.error = s.section.delete_named(active).err();
                    }),
                ),
        );
    if let Some(error) = &named.error {
        out = out.child(
            text(error.clone())
                .font(type_::LABEL)
                .color(severity_color(Severity::Error, t))
                .wrap()
                .w_full(),
        );
    }
    if let Some(path) = named
        .list
        .get(named.active)
        .and_then(|n| n.last_export.as_ref())
    {
        out = out.child(label_value(t, "Last export", path.clone()));
    }
    out
}

pub fn settings(s: &AppState, t: &Theme, k: &Kit) -> El {
    let fmt = s.shell.number_format;
    s.section.fmt.set(fmt);
    let sec = &s.section;
    let panel = col()
        .key("section-settings")
        .w_full()
        .child(heading(
            t,
            &format!(
                "Section {}",
                match sec.plane {
                    PlaneKind::Custom => "Custom",
                    kind => crate::project::kind_name(kind),
                }
            ),
        ))
        .child(col().w_full().padding(space::S2).gap(space::S2).child(note(
            t,
            "Exact planar cut of all bodies. Pick the plane and compute from the ribbon.",
        )));
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
                k,
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
            k,
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
                    button(k, "offset-dec", "-1 mm", true).on_press(move |s: &mut AppState| {
                        s.section.set_offset(offset - OFFSET_STEP_MM, now_ms())
                    }),
                )
                .child(
                    button(k, "offset-inc", "+1 mm", true).on_press(move |s: &mut AppState| {
                        s.section.set_offset(offset + OFFSET_STEP_MM, now_ms())
                    }),
                ),
        );
    if custom {
        let selected = s.model.selected.is_some();
        form = form
            .child(
                button(
                    k,
                    "align-plane",
                    "Align to selected body",
                    !busy && selected,
                )
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
    form = form.child(note(
        t,
        format!(
            "History: {} undo, {} redo",
            sec.past.len(),
            sec.future.len()
        ),
    ));

    form = form
        .child(tolerances(s, t, k))
        .child(named_sections(s, t, k));

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
                        format!(
                            "{} of {}",
                            section.components.len(),
                            imported.component_count
                        ),
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
            |s: &AppState, cx| {
                let t = rune::tok::theme(true);
                settings(s, &t, &crate::ui::kit(&t, cx.accent()))
            },
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
        assert!(
            rt.find("plane").is_none(),
            "the plane is picked in the ribbon only"
        );
        rt.state.section.set_plane(PlaneKind::Custom, 0);
        rt.invalidate();
        rt.frame(1.0 / 60.0);
        assert!(rt.find("normal-z").is_some() && rt.find("align-plane").is_some());
    }

    #[test]
    fn offset_buttons_move_the_plane_and_write_the_field() {
        let mut rt = runtime(loaded());
        press(&mut rt, "offset-inc");
        assert_eq!(rt.state.section.offset_mm, Some(16.0));
        assert_eq!(rt.state.section.offset_text.text, "16.000");
        rt.state.section.set_plane(PlaneKind::XZ, 1_000_000);
        assert_eq!(rt.state.section.offset_text.text, "");
    }

    #[test]
    fn advanced_tolerances_are_collapsed_validated_inline_and_marked_when_modified() {
        let mut rt = runtime(loaded());
        assert!(rt.find("tol-join").is_none(), "collapsed by default");
        press(&mut rt, "toggle-tolerances");
        assert!(rt.find("tol-join").is_some());
        rt.state.section.tolerance.text[2].replace_text("0.5");
        rt.state.section.commit_fields(0);
        rt.invalidate();
        rt.frame(1.0 / 60.0);
        let json = rt.accessibility_json().to_string();
        assert!(json.contains("Join distance must be between"), "{json}");
        assert!(rt.state.section.tolerance.is_default());
        rt.state.section.tolerance.text[0].replace_text("1e-3");
        rt.state.section.commit_fields(0);
        rt.invalidate();
        rt.frame(1.0 / 60.0);
        let json = rt.accessibility_json().to_string();
        assert!(json.contains("Advanced tolerances (modified)"));
        assert!(json.contains("Small region *"));
    }
}
