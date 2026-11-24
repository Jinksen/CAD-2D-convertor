//! The workspace row: model tree | splitter | viewport | splitter | properties, with each side
//! panel collapsible to a thin strip. Widths and collapsed state live in `ShellState::layout`.

use rune::tok::{Theme, space, type_};
use rune::{El, Icon, col, icon, row, spacer, text};
use rune_kit::Kit;
use rune_kit::glyphs::CARET_RIGHT;

use super::splitter::Splitter;
use super::{icons, model_tree, preview, scroller, section};
use crate::state::AppState;
use crate::state::layout;

const STRIP_WIDTH: f32 = 24.0;

/// Region title bar with a collapse button at its inner edge.
pub fn side_heading(
    t: &Theme,
    title: &str,
    key: &'static str,
    label: &'static str,
    glyph: &'static Icon,
    act: fn(&mut AppState),
) -> El {
    let button = icon(glyph, 12.0)
        .key(key)
        .role("button")
        .label(label)
        .focusable(true)
        .color(t.text_secondary)
        .on_press(move |s: &mut AppState| {
            act(s);
            s.shell.remember_layout();
        });
    let title = text(title.to_owned())
        .font(type_::SECTION)
        .color(t.text_secondary);
    row()
        .w_full()
        .px(space::S2)
        .py(space::S1)
        .items_center()
        .fill(t.control_track)
        .child(title)
        .child(spacer())
        .child(button)
}

/// What a collapsed panel leaves behind: a strip with the expand button.
fn strip(
    t: &Theme,
    region: &'static str,
    key: &'static str,
    label: &'static str,
    glyph: &'static Icon,
    act: fn(&mut AppState),
) -> El {
    col()
        .key(region)
        .role("region")
        .label(label)
        .w(STRIP_WIDTH)
        .h_full()
        .items_center()
        .border(1.0, t.separator)
        .child(
            icon(glyph, 12.0)
                .key(key)
                .role("button")
                .label(label)
                .focusable(true)
                .color(t.text_secondary)
                .mt(space::S2)
                .on_press(move |s: &mut AppState| {
                    act(s);
                    s.shell.remember_layout();
                }),
        )
}

fn tree_width(s: &AppState) -> f32 {
    s.shell.layout.tree_width
}

fn props_width(s: &AppState) -> f32 {
    s.shell.layout.props_width
}

fn properties(s: &AppState, t: &Theme, k: &Kit) -> El {
    let body = if s.model.section_selected && s.model.imported.is_some() {
        section::settings(s, t, k)
    } else {
        model_tree::properties(s, t, k)
    };
    col()
        .key("properties")
        .role("region")
        .label("Settings")
        .w(props_width(s))
        .h_full()
        .min_h(0.0)
        .border(1.0, t.separator)
        .child(side_heading(
            t,
            "Properties",
            "properties-collapse",
            "Collapse properties",
            &CARET_RIGHT,
            |s| s.shell.layout.props_collapsed = true,
        ))
        .child(scroller(k, "properties-scroll", col().w_full().child(body)))
}

pub fn workspace(s: &AppState, t: &Theme, k: &Kit) -> El {
    let l = &s.shell.layout;
    let mut r = row().key("workspace").w_full().grow(1.0).min_h(0.0);
    r = if l.tree_collapsed {
        r.child(strip(
            t,
            "model-tree",
            "tree-expand",
            "Expand model builder",
            &CARET_RIGHT,
            |s| s.shell.layout.tree_collapsed = false,
        ))
    } else {
        r.child(model_tree::tree(s, t, k)).child(
            Splitter {
                key: "split-tree",
                label: "Resize model builder",
                vertical: true,
                sign: 1.0,
                get: tree_width,
                set: |s, v| s.shell.layout.set_tree_width(v),
                reset: |s| s.shell.layout.tree_width = layout::TREE_DEFAULT,
            }
            .build(t),
        )
    };
    r = r.child(preview::viewport(s, t, k));
    if l.props_collapsed {
        r.child(strip(
            t,
            "properties",
            "properties-expand",
            "Expand properties",
            &icons::CARET_LEFT,
            |s| s.shell.layout.props_collapsed = false,
        ))
    } else {
        r.child(
            Splitter {
                key: "split-properties",
                label: "Resize properties",
                vertical: true,
                sign: -1.0,
                get: props_width,
                set: |s, v| s.shell.layout.set_props_width(v),
                reset: |s| s.shell.layout.props_width = layout::PROPS_DEFAULT,
            }
            .build(t),
        )
        .child(properties(s, t, k))
    }
}
