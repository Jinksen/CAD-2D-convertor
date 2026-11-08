//! Model tree (imported bodies, filter, visibility) and the selected body's properties.

use std::path::PathBuf;

use rune::tok::{Theme, space, type_};
use rune::{Color, El, Key, ScrollSpec, col, el, row, text, text_input};

use super::shell::open_step_dialog;
use super::{TREE_WIDTH, button, heading, label_value, rule};
use crate::api::dto::{ImportComponent, Severity};
use crate::state::AppState;

fn swatch(color: Option<[u8; 3]>, t: &Theme) -> El {
    let fill = color.map_or(t.control_off, |[r, g, b]| {
        rune::color::hex(&format!("#{r:02x}{g:02x}{b:02x}")).unwrap_or(t.control_off)
    });
    el().size(10.0, 10.0)
        .radius(2.0)
        .fill(fill)
        .border(1.0, t.separator)
        .a11y_hidden()
}

fn body_row(s: &AppState, t: &Theme, accent: Color, body: &ImportComponent, duplicate: bool) -> El {
    let selected = s.model.selected.as_deref() == Some(body.id.as_str()) && !s.shell.show_prefs;
    let hidden = s.model.hidden.contains(&body.id);
    let (select_id, eye_id) = (body.id.clone(), body.id.clone());
    let name_color = if hidden { t.text_secondary } else { t.text };
    let mut name = row().grow(1.0).gap(space::S1).child(
        text(&body.display_name)
            .font(type_::BODY)
            .color(name_color)
            .truncate(),
    );
    let label = if duplicate {
        name = name.child(
            text(&body.export_name)
                .font(type_::LABEL)
                .color(t.text_secondary)
                .truncate(),
        );
        format!("{} {}", body.display_name, body.export_name)
    } else {
        body.display_name.clone()
    };
    row()
        .key(format!("body-{}", body.id))
        .role("button")
        .label(label)
        .checked(selected)
        .focusable(true)
        .w_full()
        .gap(space::S2)
        .px(space::S2)
        .py(3.0)
        .items_center()
        .fill(if selected {
            accent.alpha(0.3)
        } else {
            Color::TRANSPARENT
        })
        .hover(t.hover)
        .on_press(move |s: &mut AppState| {
            s.shell.show_prefs = false;
            s.model.select(Some(select_id.clone()))
        })
        .child(swatch(body.source_color, t))
        .child(name)
        .child(
            text(if hidden { "hidden" } else { "shown" })
                .key(format!("eye-{}", body.id))
                .role("button")
                .label(if hidden { "Show body" } else { "Hide body" })
                .focusable(true)
                .font(type_::LABEL)
                .color(t.text_secondary)
                .on_press(move |s: &mut AppState| s.model.toggle_hidden(&eye_id)),
        )
}

/// A top-level tree node (Preferences, file, Section).
fn node(t: &Theme, accent: Color, key: &str, label: &str, on: bool) -> El {
    row()
        .key(key)
        .role("button")
        .label(label)
        .checked(on)
        .focusable(true)
        .w_full()
        .px(space::S2)
        .py(3.0)
        .fill(if on {
            accent.alpha(0.3)
        } else {
            Color::TRANSPARENT
        })
        .hover(t.hover)
        .child(text(label).font(type_::BODY).color(t.text).truncate())
}

fn filter_field(s: &AppState, t: &Theme) -> El {
    text_input(&s.model.filter, type_::BODY)
        .key("tree-filter")
        .label("Filter bodies")
        .single_line()
        .placeholder("Filter bodies (Ctrl+F)", t.text_secondary)
        .on_edit(|s: &mut AppState| &mut s.model.filter)
        // Escape clears the filter; with nothing to clear it leaves the field.
        .on_input_key(|s: &mut AppState, ev, tx| {
            if ev.key != Key::Escape {
                return false;
            }
            if s.model.filter.text.is_empty() {
                tx.blur();
            } else {
                s.model.filter.clear();
            }
            true
        })
        .w_full()
        .px(space::S2)
        .py(space::S1)
}

pub fn tree(s: &AppState, t: &Theme, accent: Color) -> El {
    let panel = col()
        .key("model-tree")
        .role("region")
        .label("Model builder")
        .w(TREE_WIDTH)
        .h_full()
        .min_h(0.0)
        .border(1.0, t.separator)
        .child(heading(t, "Model Builder"));
    let prefs = node(t, accent, "node-prefs", "Preferences", s.shell.show_prefs)
        .on_press(|s: &mut AppState| s.shell.show_prefs = !s.shell.show_prefs);
    let Some(imported) = &s.model.imported else {
        return panel.child(prefs).child(
            text("No model loaded.")
                .font(type_::BODY)
                .color(t.text_secondary)
                .padding(space::S2),
        );
    };
    let visible = s.model.visible_groups();
    let shown: usize = visible.iter().map(|(_, bodies)| bodies.len()).sum();
    let count = if s.model.filter.text.trim().is_empty() {
        imported.component_count.to_string()
    } else {
        format!("{shown} of {}", imported.component_count)
    };
    let names: Vec<&str> = imported
        .components
        .iter()
        .map(|c| c.display_name.as_str())
        .collect();
    let flat = s.model.is_flat();
    let mut rows = col().w_full();
    for (parent, bodies) in &visible {
        if !flat {
            let title = parent.rsplit(" / ").next().filter(|p| !p.is_empty());
            rows = rows.child(
                text(format!("{} ({})", title.unwrap_or("Root"), bodies.len()))
                    .font(type_::SECTION)
                    .color(t.text_secondary)
                    .px(space::S2)
                    .py(space::S1),
            );
        }
        for b in bodies {
            let duplicate = names.iter().filter(|n| **n == b.display_name).count() > 1;
            rows = rows.child(body_row(s, t, accent, b, duplicate));
        }
    }
    let file = imported.path.rsplit(['\\', '/']).next().unwrap_or("");
    let model_on = s.model.selected.is_none() && !s.shell.show_prefs;
    let mut panel = panel
        .child(filter_field(s, t))
        .child(prefs)
        .child(
            node(t, accent, "node-model", file, model_on).on_press(|s: &mut AppState| {
                s.shell.show_prefs = false;
                s.model.select(None)
            }),
        )
        .child(
            text(format!("Bodies ({count})"))
                .key("bodies-heading")
                .font(type_::SECTION)
                .color(t.text_secondary)
                .px(space::S2),
        )
        .child(rule(t));
    panel = if shown == 0 {
        panel.child(
            text("No bodies match this filter.")
                .key("no-match")
                .role("status")
                .font(type_::BODY)
                .color(t.text_secondary)
                .padding(space::S2),
        )
    } else {
        panel.child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(rows),
        )
    };
    let plane = crate::project::kind_name(s.section.plane);
    panel.child(
        node(
            t,
            accent,
            "node-section",
            &format!("Section {plane}"),
            false,
        )
        .on_press(|s: &mut AppState| s.shell.show_prefs = false),
    )
}

fn rgb(color: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

/// Property inspector: the selected body, else the imported model.
pub fn properties(s: &AppState, t: &Theme) -> El {
    let fmt = s.shell.number_format;
    let panel = col().key("body-properties").w_full();
    let fields = col().w_full().padding(space::S2).gap(space::S1);
    let Some(imported) = &s.model.imported else {
        return panel.child(heading(t, "Model")).child(step_form(s, t));
    };
    let body = imported
        .components
        .iter()
        .find(|c| s.model.selected.as_deref() == Some(c.id.as_str()));
    let Some(b) = body else {
        let bounds = &imported.bounds_mm;
        let size: Vec<f64> = (0..3)
            .map(|i| bounds.max_xyz[i] - bounds.min_xyz[i])
            .collect();
        let sha: String = imported.sha256.chars().take(12).collect();
        return panel
            .child(heading(t, "Model"))
            .child(step_form(s, t))
            .child(
                fields
                    .child(label_value(t, "Path", &imported.path))
                    .child(label_value(t, "SHA-256", format!("{sha}...")))
                    .child(label_value(
                        t,
                        "Source units",
                        imported
                            .source_unit
                            .clone()
                            .unwrap_or_else(|| "Unknown".into()),
                    ))
                    .child(label_value(
                        t,
                        "Bodies",
                        imported.component_count.to_string(),
                    ))
                    .child(label_value(t, "Size", fmt.length_product(&size))),
            );
    };
    let (id, hide_id) = (b.id.clone(), b.id.clone());
    let hidden = s.model.hidden.contains(&b.id);
    let size: Vec<f64> = (0..3)
        .map(|i| b.bounds_mm.max_xyz[i] - b.bounds_mm.min_xyz[i])
        .collect();
    let origin = b
        .name_provenance
        .map_or("Generated".to_string(), |p| format!("{p:?}").to_lowercase());
    let color = b.source_color.map_or("None in source".to_string(), |c| {
        let list = c.map(|v| v.to_string()).join(fmt.list_separator());
        let provenance = b
            .color_provenance
            .map_or("unknown".to_string(), |p| format!("{p:?}").to_lowercase());
        format!("{} RGB ({list}) {provenance}", rgb(c))
    });
    panel
        .child(heading(t, &format!("Body: {}", b.display_name)))
        .child(
            fields
                .child(label_value(t, "Export name", &b.export_name))
                .child(label_value(
                    t,
                    "Source name",
                    b.source_name.clone().unwrap_or_else(|| "none".into()),
                ))
                .child(label_value(
                    t,
                    "Assembly path",
                    b.hierarchy_path.join(" / "),
                ))
                .child(label_value(t, "Name origin", origin))
                .child(label_value(t, "Color", color))
                .child(label_value(t, "Size", fmt.length_product(&size)))
                .child(label_value(t, "Body ordinal", b.body_ordinal.to_string()))
                .child(
                    row()
                        .gap(space::S1)
                        .child(
                            button(
                                t,
                                "hide-body",
                                if hidden { "Show (V)" } else { "Hide (V)" },
                                true,
                            )
                            .on_press(move |s: &mut AppState| s.model.toggle_hidden(&hide_id)),
                        )
                        .child(
                            button(t, "isolate-body", "Isolate (I)", true)
                                .on_press(move |s: &mut AppState| s.model.toggle_isolate(&id)),
                        ),
                ),
        )
}

/// STEP path field with Import and Browse.
fn step_form(s: &AppState, t: &Theme) -> El {
    let idle = s.shell.busy.is_none();
    col()
        .w_full()
        .padding(space::S2)
        .gap(space::S1)
        .child(
            text_input(&s.shell.step_path, type_::BODY)
                .key("step-path")
                .label("STEP path")
                .single_line()
                .placeholder("C:\\Projects\\motor.step", t.text_secondary)
                .on_edit(|s: &mut AppState| &mut s.shell.step_path)
                .w_full(),
        )
        .child(
            row()
                .gap(space::S1)
                .child(button(t, "import-path", "Import STEP", idle).on_press(
                    |s: &mut AppState| {
                        let path = s.shell.step_path.text.trim().to_owned();
                        if path.is_empty() {
                            s.shell.log(Severity::Error, "Enter a STEP path.");
                        } else {
                            s.open_step(PathBuf::from(path));
                        }
                    },
                ))
                .child(button(t, "browse-step", "Browse...", idle).on_press_tx(open_step_dialog)),
        )
}
