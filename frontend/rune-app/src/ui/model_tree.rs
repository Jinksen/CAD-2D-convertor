//! Model tree (imported bodies, filter, visibility) and the selected body's properties.

use std::path::PathBuf;
use std::rc::Rc;

use rune::tok::{Theme, space, type_};
use rune::{El, Key, col, icon, row, text};
use rune_kit::glyphs;
use rune_kit::{Kit, TextInput};

use super::groups::{self, folder_rows, group_rows, new_group_form};
use super::panels::side_heading;
use super::shell::open_step_dialog;
use super::tree_menu::attach;
use super::tree_row::{Glyph, Item, row_el};
use super::{button, heading, icons, label_value, rule, scroller};
use crate::api::dto::{ImportComponent, Severity};
use crate::state::AppState;
use crate::state::selection::Node as TreeNode;

pub(super) fn body_row(
    s: &AppState,
    t: &Theme,
    k: &Kit,
    body: &ImportComponent,
    duplicate: bool,
    depth: usize,
    node: TreeNode,
) -> El {
    let m = &s.model;
    let on = m.is_picked(&body.id) && !m.section_selected && !s.shell.show_prefs;
    let hidden = m.hidden.contains(&body.id);
    let (eye_id, hover_id) = (body.id.clone(), body.id.clone());

    let (press_node, drag_node, menu_node) = (node.clone(), node.clone(), node);
    let mut n = Item::new(
        format!("body-{}", body.id),
        &body.display_name,
        &icons::BODY,
    );
    n.aria = if duplicate {
        format!("{} {}", body.display_name, body.export_name)
    } else {
        body.display_name.clone()
    };
    n.extra = duplicate.then(|| body.export_name.clone());
    (n.depth, n.swatch, n.on, n.dim) = (depth, Some(body.source_color), on, hidden);
    n.lit = m.hover.as_deref() == Some(body.id.as_str());
    n.mark = filter_mark(&body.display_name, &m.filter.text);
    n.status = if hidden {
        Some("hidden")
    } else if m.isolated {
        Some("isolated")
    } else {
        None
    };
    n.eye = Some((
        Glyph {
            key: format!("eye-{}", body.id),
            label: if hidden { "Show body" } else { "Hide body" }.into(),
            act: Rc::new(move |s: &mut AppState| s.model.toggle_hidden(&eye_id)),
        },
        hidden,
    ));
    let row = row_el(n, t, k)
        .on_press_tx(move |s: &mut AppState, tx| {
            let mods = tx.mods();
            s.shell.show_prefs = false;
            s.model
                .click_body(&press_node, mods.ctrl || mods.cmd, mods.shift)
        })
        .on_drag(move |s: &mut AppState, d, _| super::tree_menu::drag_body(s, &drag_node, d))
        .on_hover_change(move |s: &mut AppState, over, _| {
            if over {
                s.model.hover = Some(hover_id.clone());
            } else if s.model.hover.as_deref() == Some(hover_id.as_str()) {
                s.model.hover = None;
            }
        });
    super::tree_menu::attach(row, menu_node)
}

/// Byte range of `name` matching the filter, when lowercasing keeps byte offsets.
fn filter_mark(name: &str, filter: &str) -> Option<(usize, usize)> {
    let needle = filter.trim().to_lowercase();
    let lower = name.to_lowercase();
    if needle.is_empty() || lower.len() != name.len() {
        return None;
    }
    lower.find(&needle).map(|a| (a, a + needle.len()))
}

/// "XY, z = 37.500 mm": the section plane and offset as the Section node shows them.
fn section_label(s: &AppState) -> String {
    let Some(imported) = &s.model.imported else {
        return "Section".into();
    };
    let sec = &s.section;
    let offset = sec.effective_offset(&imported.bounds_mm);
    let plane = match sec.plane {
        crate::state::section::PlaneKind::Custom => "Custom",
        kind => crate::project::kind_name(kind),
    };
    format!(
        "{plane}, {} = {}",
        super::section::plane_axis(sec.plane),
        s.shell.number_format.length(offset)
    )
}

fn filter_field(s: &AppState, k: &Kit, t: &Theme) -> El {
    let input = TextInput::new(&s.model.filter)
        .label("Filter bodies")
        .placeholder("Filter bodies (Ctrl+F)")
        .width(None)
        .height(22.0)
        .on_edit(|s: &mut AppState| &mut s.model.filter)
        // Escape clears the filter; with nothing to clear it leaves the field.
        .on_key(|s: &mut AppState, ev, tx| {
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
        .build(k)
        .key("tree-filter");
    let mut r = row()
        .w_full()
        .px(space::S2)
        .py(space::S1)
        .gap(space::S1)
        .items_center()
        .child(input.grow(1.0));
    if !s.model.filter.text.trim().is_empty() {
        let n = s.model.visible_bodies().len();
        r = r
            .child(
                text(if n == 1 {
                    "1 match".to_string()
                } else {
                    format!("{n} matches")
                })
                .key("filter-count")
                .role("status")
                .font(type_::LABEL)
                .color(t.text_secondary),
            )
            .child(
                icon(&glyphs::CLOSE, 12.0)
                    .key("filter-clear")
                    .role("button")
                    .label("Clear filter")
                    .focusable(true)
                    .color(t.text_secondary)
                    .on_press(|s: &mut AppState| s.model.filter.clear()),
            );
    }
    r
}

fn tree_tools(k: &Kit) -> El {
    row()
        .key("tree-tools")
        .role("toolbar")
        .label("Tree tools")
        .w_full()
        .px(space::S2)
        .gap(space::S1)
        .child(
            button(k, "expand-all", "Expand all", true)
                .on_press(|s: &mut AppState| s.model.expand_all()),
        )
        .child(
            button(k, "collapse-all", "Collapse all", true)
                .on_press(|s: &mut AppState| s.model.collapse_all()),
        )
}

pub fn tree(s: &AppState, t: &Theme, k: &Kit) -> El {
    let panel = col()
        .key("model-tree")
        .role("region")
        .label("Model builder")
        .w(s.shell.layout.tree_width)
        .h_full()
        .min_h(0.0)
        .border(1.0, t.separator)
        .child(side_heading(
            t,
            "Model Builder",
            "tree-collapse",
            "Collapse model builder",
            &icons::CARET_LEFT,
            |s| s.shell.layout.tree_collapsed = true,
        ));
    let Some(imported) = &s.model.imported else {
        return panel.child(
            text("No model loaded.")
                .font(type_::BODY)
                .color(t.text_secondary)
                .padding(space::S2),
        );
    };
    let visible = s.model.visible_tree();
    let shown = visible.count();
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
    let dup = |name: &str| names.iter().filter(|n| **n == name).count() > 1;
    let file = imported.path.rsplit(['\\', '/']).next().unwrap_or("");
    let m = &s.model;
    let model_on = m.selected.is_none() && m.selected_group.is_none() && !m.section_selected;
    let model_open = !m.is_collapsed("model");
    let bodies_open = !m.is_collapsed("bodies");
    let mut file_node = Item::new("node-model", file, &glyphs::FILE);
    (file_node.open, file_node.on) = (Some(model_open), model_on && !s.shell.show_prefs);
    file_node.chevron_act = Some(Glyph {
        key: "model-toggle".into(),
        label: if model_open {
            "Collapse model"
        } else {
            "Expand model"
        }
        .into(),
        act: Rc::new(|s: &mut AppState| s.model.toggle_collapsed("model")),
    });
    let mut rows = col()
        .w_full()
        .child(row_el(file_node, t, k).on_press(|s: &mut AppState| {
            s.shell.show_prefs = false;
            s.model.select(None)
        }));
    if !model_open {
        return panel
            .child(tree_tools(k))
            .child(filter_field(s, k, t))
            .child(rule(t))
            .child(scroller(k, "tree-scroll", rows));
    }
    let mut sec = Item::new("node-section", section_label(s), &icons::SECTION);
    (sec.depth, sec.on) = (1, m.section_selected && !s.shell.show_prefs);
    sec.aria = format!("Section {}", sec.name);
    rows = rows.child(attach(
        row_el(sec, t, k).on_press(|s: &mut AppState| {
            s.shell.show_prefs = false;
            s.model.activate(&TreeNode::Section)
        }),
        TreeNode::Section,
    ));
    let mut head = Item::new(
        "bodies-heading",
        format!("Bodies ({count})"),
        &icons::ASSEMBLY,
    );
    (head.depth, head.open) = (1, Some(bodies_open));
    rows = rows
        .child(row_el(head, t, k).on_press(|s: &mut AppState| s.model.toggle_collapsed("bodies")));
    rows = if !bodies_open {
        rows
    } else if shown == 0 {
        rows.child(
            text("No bodies match this filter.")
                .key("no-match")
                .role("status")
                .font(type_::BODY)
                .color(t.text_secondary)
                .padding(space::S2),
        )
    } else {
        folder_rows(s, t, k, &visible, 2, &dup, rows)
    };
    let rows = group_rows(s, t, k, &dup, rows);
    panel
        .child(tree_tools(k))
        .child(filter_field(s, k, t))
        .child(rule(t))
        .child(scroller(k, "tree-scroll", rows))
}

fn rgb(color: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

/// Property inspector: the selected body, else the imported model.
pub fn properties(s: &AppState, t: &Theme, k: &Kit) -> El {
    let fmt = s.shell.number_format;
    let panel = col().key("body-properties").w_full();
    let fields = col().w_full().padding(space::S2).gap(space::S1);
    let Some(imported) = &s.model.imported else {
        return panel.child(heading(t, "Model")).child(step_form(s, k));
    };
    if let Some(name) = &s.model.selected_group {
        return groups::properties(s, t, k, name);
    }
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
            .child(heading(
                t,
                &format!(
                    "Model {}",
                    imported.path.rsplit(['\\', '/']).next().unwrap_or("")
                ),
            ))
            .child(step_form(s, k))
            .child(
                fields
                    .child(label_value(t, "Path", &imported.path))
                    .child(label_value(t, "SHA-256", format!("{sha}…")))
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
                    .child(label_value(t, "Size", fmt.length_product(&size)))
                    .child(
                        text("Groups")
                            .font(type_::SECTION)
                            .color(t.text_secondary)
                            .pt(space::S2),
                    )
                    .child(new_group_form(s, k)),
            );
    };
    let (id, hide_id) = (b.id.clone(), b.id.clone());
    let groups = groups::membership(s, t, k, &b.id);
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
        .child(heading(t, &format!("Body {}", b.display_name)))
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
                                k,
                                "hide-body",
                                if hidden { "Show (V)" } else { "Hide (V)" },
                                true,
                            )
                            .on_press(move |s: &mut AppState| s.model.toggle_hidden(&hide_id)),
                        )
                        .child(
                            button(k, "isolate-body", "Isolate (I)", true)
                                .on_press(move |s: &mut AppState| s.model.toggle_isolate(&id)),
                        ),
                )
                .child(groups)
                .children(super::regions::rows(s, t, &b.id)),
        )
}

/// STEP path field with Import and Browse.
fn step_form(s: &AppState, k: &Kit) -> El {
    let idle = s.shell.busy.is_none();
    col()
        .w_full()
        .padding(space::S2)
        .gap(space::S1)
        .child(
            TextInput::new(&s.shell.step_path)
                .label("STEP path")
                .placeholder("C:\\Projects\\motor.step")
                .width(None)
                .height(22.0)
                .on_edit(|s: &mut AppState| &mut s.shell.step_path)
                .build(k)
                .key("step-path"),
        )
        .child(
            row()
                .gap(space::S1)
                .child(button(k, "import-path", "Import STEP", idle).on_press(
                    |s: &mut AppState| {
                        let path = s.shell.step_path.text.trim().to_owned();
                        if path.is_empty() {
                            s.shell.log(Severity::Error, "Enter a STEP path.");
                        } else {
                            s.open_step(PathBuf::from(path));
                        }
                    },
                ))
                .child(button(k, "browse-step", "Browse…", idle).on_press_tx(open_step_dialog)),
        )
}
