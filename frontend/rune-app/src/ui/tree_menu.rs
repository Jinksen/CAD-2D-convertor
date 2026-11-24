//! Model-tree context menu and the drag-a-body-onto-a-group gesture.

use rune::{Drag, El, Phase, PointerPress, el};
use rune_kit::{Kit, MenuEntry, MenuItem, context_menu};

use super::tree_row::ROW_H;
use crate::state::AppState;
use crate::state::selection::{ContextMenu, Node};

/// Right-click opens the menu for this row, selecting it first unless it is already picked.
pub fn attach(row: El, node: Node) -> El {
    row.on_secondary_press(move |s: &mut AppState, p: PointerPress, _| {
        let picked = node
            .body_id()
            .is_some_and(|id| s.model.multi.len() > 1 && s.model.multi.contains(id));
        if !picked {
            s.model.activate(&node);
        }
        s.shell.show_prefs = false;
        s.model.menu = Some(ContextMenu {
            node: node.clone(),
            x: p.x,
            y: p.y,
            sub_open: false,
        });
    })
}

/// Group under the dragged body row, found from the pointer travel: tree rows have one height.
fn group_under(s: &AppState, from: &Node, d: &Drag) -> Option<String> {
    let rows = s.model.rows();
    let at = rows.iter().position(|(n, _)| n == from)?;
    let delta = ((d.start_y + d.dy) / ROW_H).floor() as isize;
    match &rows.get(at.checked_add_signed(delta)?)?.0 {
        Node::Group(g) | Node::Member(g, _) => Some(g.clone()),
        _ => None,
    }
}

/// Dropping a body row on a group adds it (or all picked bodies) to that group.
pub fn drag_body(s: &mut AppState, from: &Node, d: &mut Drag) {
    let target = group_under(s, from, d);
    match d.phase {
        Phase::Move => s.model.drop_target = target,
        Phase::Up => {
            s.model.drop_target = None;
            let (Some(group), Some(id)) = (target, from.body_id()) else {
                return;
            };
            if d.cancelled() {
                return;
            }
            let mut ids = s.model.targets();
            if !ids.iter().any(|i| i == id) {
                ids = vec![id.to_owned()];
            }
            s.model.add_to_group(&group, &ids);
        }
        Phase::Down => {}
    }
}

fn close() -> impl Fn(&mut AppState) + Clone + 'static {
    |s: &mut AppState| s.model.menu = None
}

/// Runs `f`, then closes the menu.
fn pick(item: MenuItem, f: impl Fn(&mut AppState) + 'static) -> MenuItem {
    item.on_select(move |s: &mut AppState| {
        f(s);
        s.model.menu = None;
    })
}

fn new_group_name(s: &AppState) -> String {
    (1..)
        .map(|n| format!("Group {n}"))
        .find(|name| s.model.group(name).is_none())
        .unwrap_or_default()
}

fn body_items(
    s: &AppState,
    menu: &ContextMenu,
    id: &str,
    member_of: Option<&str>,
) -> Vec<MenuEntry> {
    let ids = if s.model.multi.len() > 1 && s.model.multi.contains(id) {
        s.model.targets()
    } else {
        vec![id.to_owned()]
    };
    let hidden = ids.iter().all(|i| s.model.hidden.contains(i));
    let (a, b, c) = (ids.clone(), ids.clone(), ids.clone());
    let mut groups: Vec<MenuEntry> = s
        .model
        .groups
        .iter()
        .map(|g| {
            let (name, ids) = (g.name.clone(), ids.clone());
            MenuEntry::item(pick(MenuItem::new(&g.name), move |s| {
                s.model.add_to_group(&name, &ids)
            }))
        })
        .collect();
    groups.push(MenuEntry::item(pick(
        MenuItem::new("New group"),
        move |s| {
            let name = new_group_name(s);
            s.model.new_group = rune::TextEdit::new(&name);
            s.model.selected = c.first().cloned();
            if s.model.create_group().is_ok() {
                s.model.add_to_group(&name, &c);
                s.model.start_rename(&name);
            }
        },
    )));
    let zoom = id.to_owned();
    let mut items = vec![
        MenuEntry::item(pick(
            MenuItem::new(if hidden { "Show" } else { "Hide" }).shortcut("V"),
            move |s| s.model.toggle_hidden_many(&a),
        )),
        MenuEntry::item(pick(MenuItem::new("Isolate").shortcut("I"), move |s| {
            s.model.isolate(&b)
        })),
        MenuEntry::item(pick(MenuItem::new("Zoom to").shortcut("F"), move |s| {
            s.zoom_to(&zoom)
        })),
        MenuEntry::item(
            MenuItem::new("Add to group")
                .submenu(groups, menu.sub_open)
                .on_select(|s: &mut AppState| {
                    if let Some(m) = &mut s.model.menu {
                        m.sub_open = !m.sub_open;
                    }
                }),
        ),
    ];
    if let Some(group) = member_of {
        let (group, id) = (group.to_owned(), id.to_owned());
        items.push(MenuEntry::item(pick(
            MenuItem::new("Remove from group"),
            move |s| s.model.toggle_member(&group, &id),
        )));
    }
    items
}

fn entries(s: &AppState, menu: &ContextMenu) -> Vec<MenuEntry> {
    let node = menu.node.clone();
    let mut items = match &menu.node {
        Node::Body(id) => body_items(s, menu, id, None),
        Node::Member(g, id) => body_items(s, menu, id, Some(g)),
        Node::Group(name) => {
            let hidden = s.model.group_hidden(name);
            let (a, b, c, d) = (name.clone(), name.clone(), name.clone(), name.clone());
            vec![
                MenuEntry::item(pick(
                    MenuItem::new(if hidden { "Show" } else { "Hide" }).shortcut("V"),
                    move |s| s.model.toggle_group_hidden(&a),
                )),
                MenuEntry::item(pick(MenuItem::new("Isolate").shortcut("I"), move |s| {
                    s.model.toggle_group_isolate(&b)
                })),
                MenuEntry::item(pick(MenuItem::new("Rename").shortcut("F2"), move |s| {
                    s.model.start_rename(&c)
                })),
                MenuEntry::item(pick(MenuItem::new("Delete group").danger(true), move |s| {
                    s.model.delete_group(&d)
                })),
            ]
        }
        Node::Model | Node::Section => vec![],
        other => {
            let key = other.collapse_key().unwrap_or_default();
            let open = !s.model.collapsed.contains(&key);
            vec![MenuEntry::item(pick(
                MenuItem::new(if open { "Collapse" } else { "Expand" }),
                move |s| s.model.toggle_collapsed(&key),
            ))]
        }
    };
    if matches!(
        node,
        Node::Model | Node::Section | Node::Body(_) | Node::Member(..) | Node::Group(_)
    ) {
        items.push(MenuEntry::Separator);
        items.push(MenuEntry::item(pick(
            MenuItem::new("Properties").shortcut("Enter"),
            move |s| s.model.activate(&node),
        )));
    }
    items
}

/// The open context menu as overlay layers for `rune_kit::root`.
pub fn overlay(s: &AppState, k: &Kit) -> Vec<El> {
    let Some(menu) = &s.model.menu else {
        return vec![];
    };
    let mut layer = vec![];
    let mut m = context_menu(
        "tree-menu",
        el(),
        Some(PointerPress {
            x: menu.x,
            y: menu.y,
        }),
    )
    .on_close(close());
    for entry in entries(s, menu) {
        m = match entry {
            MenuEntry::Item(item) => m.item(*item),
            MenuEntry::Separator => m.separator(),
            MenuEntry::Label(l) => m.group_label(l),
        };
    }
    let trigger = m.build(k, &mut layer);
    layer.push(trigger);
    layer
}
