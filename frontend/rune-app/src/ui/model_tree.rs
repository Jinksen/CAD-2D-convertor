//! Model tree (imported bodies, filter, visibility) and the selected body's properties.

use rune::tok::{Theme, space, type_};
use rune::{Color, El, ScrollSpec, col, el, row, text, text_input};

use super::{TREE_WIDTH, button, heading, label_value, rule};
use crate::api::dto::ImportComponent;
use crate::state::AppState;

fn swatch(color: Option<[u8; 3]>, t: &Theme) -> El {
    let fill = color
        .and_then(|[r, g, b]| rune::color::hex(&format!("#{r:02x}{g:02x}{b:02x}")))
        .unwrap_or(t.control_off);
    el().size(10.0, 10.0)
        .radius(2.0)
        .fill(fill)
        .border(1.0, t.separator)
}

fn body_row(s: &AppState, t: &Theme, accent: Color, body: &ImportComponent, duplicate: bool) -> El {
    let selected = s.model.selected.as_deref() == Some(body.id.as_str());
    let hidden = s.model.hidden.contains(&body.id);
    let (select_id, eye_id) = (body.id.clone(), body.id.clone());
    let name_color = if hidden { t.text_secondary } else { t.text };
    let mut name = row().grow(1.0).gap(space::S1).child(
        text(&body.display_name)
            .font(type_::BODY)
            .color(name_color)
            .truncate(),
    );
    if duplicate {
        name = name.child(
            text(&body.export_name)
                .font(type_::LABEL)
                .color(t.text_secondary)
                .truncate(),
        );
    }
    row()
        .key(format!("body-{}", body.id))
        .role("button")
        .label(&body.display_name)
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
        .on_press(move |s: &mut AppState| s.model.select(Some(select_id.clone())))
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

pub fn tree(s: &AppState, t: &Theme, accent: Color) -> El {
    let visible = s.model.visible_bodies();
    let mut panel = col()
        .key("model-tree")
        .role("region")
        .label("Model builder")
        .w(TREE_WIDTH)
        .h_full()
        .min_h(0.0)
        .border(1.0, t.separator)
        .child(heading(t, "Model Builder"));
    let Some(imported) = &s.model.imported else {
        return panel.child(
            text("Open or drop a STEP file to list its bodies.")
                .font(type_::BODY)
                .color(t.text_secondary)
                .padding(space::S2),
        );
    };
    let count = if visible.len() == imported.components.len() {
        imported.component_count.to_string()
    } else {
        format!("{} of {}", visible.len(), imported.component_count)
    };
    let names: Vec<&str> = imported
        .components
        .iter()
        .map(|c| c.display_name.as_str())
        .collect();
    let rows = visible.iter().map(|b| {
        body_row(
            s,
            t,
            accent,
            b,
            names.iter().filter(|n| **n == b.display_name).count() > 1,
        )
    });
    panel = panel
        .child(
            text_input(&s.model.filter, type_::BODY)
                .key("tree-filter")
                .label("Filter bodies")
                .single_line()
                .placeholder("Filter bodies", t.text_secondary)
                .on_edit(|s: &mut AppState| &mut s.model.filter)
                .w_full()
                .px(space::S2)
                .py(space::S1),
        )
        .child(
            text(format!("Bodies ({count})"))
                .font(type_::SECTION)
                .color(t.text_secondary)
                .px(space::S2),
        )
        .child(rule(t));
    if visible.is_empty() {
        return panel.child(
            text("No bodies match this filter.")
                .font(type_::BODY)
                .color(t.text_secondary)
                .padding(space::S2),
        );
    }
    panel.child(
        col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .children(rows),
    )
}

/// Property inspector of the selected body.
pub fn properties(s: &AppState, t: &Theme) -> El {
    let fmt = s.shell.number_format;
    let mut panel = col()
        .key("body-properties")
        .w_full()
        .child(heading(t, "Body properties"));
    let body = s
        .model
        .imported
        .iter()
        .flat_map(|i| &i.components)
        .find(|c| s.model.selected.as_deref() == Some(c.id.as_str()));
    let Some(b) = body else {
        return panel.child(
            text("Select a body in the model tree.")
                .font(type_::BODY)
                .color(t.text_secondary)
                .padding(space::S2),
        );
    };
    let id = b.id.clone();
    let size: Vec<String> = (0..3)
        .map(|i| fmt.length(b.bounds_mm.max_xyz[i] - b.bounds_mm.min_xyz[i]))
        .collect();
    let color = b.source_color.map_or("none".to_string(), |[r, g, bl]| {
        format!("#{r:02x}{g:02x}{bl:02x}")
    });
    panel = panel.child(
        col()
            .w_full()
            .padding(space::S2)
            .gap(space::S1)
            .child(label_value(t, "Name", &b.display_name))
            .child(label_value(t, "Export name", &b.export_name))
            .child(label_value(
                t,
                "Source name",
                b.source_name.clone().unwrap_or_else(|| "none".into()),
            ))
            .child(label_value(t, "Path", b.hierarchy_path.join(" / ")))
            .child(label_value(t, "Color", color))
            .child(label_value(t, "Size (mm)", size.join(" x ")))
            .child(
                button(t, "isolate-body", "Isolate / show all", true)
                    .on_press(move |s: &mut AppState| s.model.toggle_isolate(&id)),
            ),
    );
    panel
}
