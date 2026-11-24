//! The one row primitive of the model tree: indent guides, chevron, icon, ellipsized name with
//! a tooltip, muted count, status label and an eye toggle. Callers attach `on_press`.

use std::rc::Rc;

use rune::tok::{Theme, space, type_};
use rune::{Color, El, Icon, el, icon, row, text};
use rune_kit::Kit;
use rune_kit::glyphs::{CARET_RIGHT, EYE, EYE_OFF, TRIANGLE_DOWN};

use super::selected;
use crate::state::AppState;

pub const ROW_H: f32 = 22.0;
const INDENT: f32 = 14.0;
const SLOT: f32 = 14.0;

pub type Action = Rc<dyn Fn(&mut AppState)>;

/// A pressable glyph (chevron or eye) inside a row.
pub struct Glyph {
    pub key: String,
    pub label: String,
    pub act: Action,
}

pub struct Node {
    pub key: String,
    /// Accessible name of the row.
    pub aria: String,
    pub name: String,
    /// Dim second name (the export name of duplicate bodies).
    pub extra: Option<String>,
    pub depth: usize,
    pub icon: &'static Icon,
    pub swatch: Option<Option<[u8; 3]>>,
    pub count: Option<usize>,
    /// `Some(open)` draws a chevron; `chevron_act` makes it a button of its own.
    pub open: Option<bool>,
    pub chevron_act: Option<Glyph>,
    pub on: bool,
    pub dim: bool,
    /// "hidden" or "isolated": status is text, never colour alone.
    pub status: Option<&'static str>,
    /// Eye toggle; `hidden` selects the closed-eye glyph and keeps it visible.
    pub eye: Option<(Glyph, bool)>,
}

impl Node {
    pub fn new(key: impl Into<String>, name: impl Into<String>, icon: &'static Icon) -> Node {
        let name = name.into();
        Node {
            key: key.into(),
            aria: name.clone(),
            name,
            extra: None,
            depth: 0,
            icon,
            swatch: None,
            count: None,
            open: None,
            chevron_act: None,
            on: false,
            dim: false,
            status: None,
            eye: None,
        }
    }
}

fn swatch(color: Option<[u8; 3]>, t: &Theme) -> El {
    let fill = color.map_or(t.control_off, |[r, g, b]| {
        rune::color::hex(&format!("#{r:02x}{g:02x}{b:02x}")).unwrap_or(t.control_off)
    });
    el().size(8.0, 8.0)
        .radius(2.0)
        .fill(fill)
        .border(1.0, t.separator)
        .a11y_hidden()
}

fn guides(depth: usize, t: &Theme) -> El {
    let mut g = row().h_full().ml(space::S1);
    for _ in 0..depth {
        g = g.child(
            row()
                .w(INDENT)
                .h_full()
                .child(el().w(1.0).h_full().ml(SLOT / 2.0).fill(t.separator)),
        );
    }
    g
}

fn glyph_button(g: &Glyph, i: &'static Icon, t: &Theme) -> El {
    let act = g.act.clone();
    icon(i, 12.0)
        .key(g.key.clone())
        .role("button")
        .label(g.label.clone())
        .focusable(true)
        .color(t.text_secondary)
        .on_press(move |s: &mut AppState| act(s))
}

pub fn row_el(n: Node, t: &Theme, k: &Kit) -> El {
    let ink = if n.dim { t.text_secondary } else { t.text };
    let slot = match (n.open, &n.chevron_act) {
        (Some(open), Some(g)) => {
            glyph_button(g, if open { &TRIANGLE_DOWN } else { &CARET_RIGHT }, t)
        }
        (Some(open), None) => icon(if open { &TRIANGLE_DOWN } else { &CARET_RIGHT }, 12.0)
            .color(t.text_secondary)
            .a11y_hidden(),
        _ => el().size(12.0, 12.0),
    };
    let mut name = row().grow(1.0).min_w(0.0).gap(space::S1).child(
        text(&n.name)
            .font(type_::BODY)
            .color(ink)
            .truncate()
            .help_when_truncated(match &n.extra {
                Some(extra) => format!("{} ({extra})", n.name),
                None => n.name.clone(),
            }),
    );
    if let Some(extra) = &n.extra {
        name = name.child(
            text(extra)
                .font(type_::LABEL)
                .color(t.text_secondary)
                .truncate(),
        );
    }
    let mut r = row()
        .key(n.key)
        .role("button")
        .label(n.aria)
        .checked(n.on)
        .focusable(true)
        .w_full()
        .h(ROW_H)
        .pr(space::S2)
        .gap(space::S1)
        .items_center()
        .hover_group_focus()
        .fill(if n.on {
            selected(k)
        } else {
            Color::TRANSPARENT
        })
        .hover(t.hover)
        .child(guides(n.depth, t))
        .child(slot)
        .child(icon(n.icon, 14.0).color(ink).a11y_hidden());
    if let Some(color) = n.swatch {
        r = r.child(swatch(color, t));
    }
    r = r.child(name);
    if let Some(status) = n.status {
        r = r.child(text(status).font(type_::LABEL).color(t.text_secondary));
    }
    if let Some(count) = n.count {
        r = r.child(
            text(count.to_string())
                .font(type_::LABEL)
                .color(t.text_secondary),
        );
    }
    if let Some((g, hidden)) = &n.eye {
        let eye = glyph_button(g, if *hidden { &EYE_OFF } else { &EYE }, t);
        r = r.child(if *hidden {
            eye
        } else {
            eye.group_opacity(0.0, 1.0)
        });
    }
    r
}
