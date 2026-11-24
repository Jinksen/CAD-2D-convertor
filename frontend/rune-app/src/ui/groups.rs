//! Assembly folders and user groups in the model tree, and the group property pane.

use std::rc::Rc;

use rune::tok::{Theme, space, type_};
use rune::{El, col, row, text};
use rune_kit::{Kit, TextInput};

use super::model_tree::body_row;
use super::tree_row::{Action, Glyph, Node, row_el};
use super::{button, heading, icons, label_value};
use crate::api::dto::Severity;
use crate::state::AppState;
use crate::state::model::group_key;
use crate::state::tree::Folder;

fn bodies(count: usize) -> String {
    if count == 1 {
        "1 body".into()
    } else {
        format!("{count} bodies")
    }
}

fn toggle(key: &str) -> Action {
    let key = key.to_owned();
    Rc::new(move |s: &mut AppState| s.model.toggle_collapsed(&key))
}

/// Bodies of `folder`, then its subfolders, each subfolder collapsible.
pub fn folder_rows(
    s: &AppState,
    t: &Theme,
    k: &Kit,
    folder: &Folder,
    depth: usize,
    dup: &dyn Fn(&str) -> bool,
    mut rows: El,
) -> El {
    for b in &folder.bodies {
        rows = rows.child(body_row(s, t, k, b, dup(&b.display_name), depth));
    }
    for sub in &folder.folders {
        let open = !s.model.is_collapsed(&sub.key);
        let mut n = Node::new(format!("folder-{}", sub.key), &sub.name, &icons::ASSEMBLY);
        n.aria = format!(
            "{}, {}, {}",
            sub.name,
            bodies(sub.count()),
            if open { "expanded" } else { "collapsed" }
        );
        (n.depth, n.count, n.open) = (depth, Some(sub.count()), Some(open));
        let act = toggle(&sub.key);
        rows = rows.child(row_el(n, t, k).on_press(move |s: &mut AppState| act(s)));
        if open {
            rows = folder_rows(s, t, k, sub, depth + 1, dup, rows);
        }
    }
    rows
}

/// The "Groups" node: one collapsible node per group with its matching members.
pub fn group_rows(
    s: &AppState,
    t: &Theme,
    k: &Kit,
    dup: &dyn Fn(&str) -> bool,
    mut rows: El,
) -> El {
    if s.model.groups.is_empty() {
        return rows;
    }
    let all_open = !s.model.is_collapsed("groups");
    let mut head = Node::new(
        "groups-heading",
        format!("Groups ({})", s.model.groups.len()),
        &icons::GROUP,
    );
    (head.depth, head.open) = (1, Some(all_open));
    let act = toggle("groups");
    rows = rows.child(row_el(head, t, k).on_press(move |s: &mut AppState| act(s)));
    if !all_open {
        return rows;
    }
    for group in &s.model.groups {
        let members = s.model.visible_members(group);
        let collapse = group_key(&group.name);
        let open = !s.model.is_collapsed(&collapse);
        let on =
            s.model.selected_group.as_deref() == Some(group.name.as_str()) && !s.shell.show_prefs;
        let (select, hide) = (group.name.clone(), group.name.clone());
        let hidden = s.model.group_hidden(&group.name);
        let count = group.component_ids.len();
        let mut n = Node::new(format!("group-{}", group.name), &group.name, &icons::GROUP);
        n.aria = format!("Group {}, {}", group.name, bodies(count));
        (n.depth, n.count, n.open, n.on, n.dim) = (2, Some(count), Some(open), on, hidden);
        n.status = hidden.then_some("hidden");
        n.chevron_act = Some(Glyph {
            key: format!("group-toggle-{}", group.name),
            label: if open {
                "Collapse group"
            } else {
                "Expand group"
            }
            .into(),
            act: toggle(&collapse),
        });
        n.eye = Some((
            Glyph {
                key: format!("group-eye-{}", group.name),
                label: if hidden { "Show group" } else { "Hide group" }.into(),
                act: Rc::new(move |s: &mut AppState| s.model.toggle_group_hidden(&hide)),
            },
            hidden,
        ));
        rows = rows.child(row_el(n, t, k).on_press(move |s: &mut AppState| {
            s.shell.show_prefs = false;
            s.model.select_group(&select)
        }));
        if open {
            for b in members {
                rows = rows.child(body_row(s, t, k, b, dup(&b.display_name), 3));
            }
        }
    }
    rows
}

fn report(s: &mut AppState, result: Result<(), String>) {
    if let Err(error) = result {
        s.shell.log(Severity::Error, error);
    }
}

/// Name field and "New group" button; the group holds the selected body, if any.
pub fn new_group_form(s: &AppState, k: &Kit) -> El {
    row()
        .w_full()
        .gap(space::S1)
        .child(
            TextInput::new(&s.model.new_group)
                .label("New group name")
                .placeholder("Group name")
                .width(None)
                .height(22.0)
                .on_edit(|s: &mut AppState| &mut s.model.new_group)
                .build(k)
                .key("new-group-name")
                .grow(1.0),
        )
        .child(
            button(k, "new-group", "New group", true).on_press(|s: &mut AppState| {
                let result = s.model.create_group();
                report(s, result)
            }),
        )
}

/// Group buttons in the body pane: add to or remove from each group, or start a new one.
pub fn membership(s: &AppState, t: &Theme, k: &Kit, id: &str) -> El {
    let mut list = col().w_full().gap(space::S1).child(
        text("Groups")
            .font(type_::SECTION)
            .color(t.text_secondary)
            .pt(space::S2),
    );
    for group in &s.model.groups {
        let member = group.component_ids.iter().any(|m| m == id);
        let (name, id) = (group.name.clone(), id.to_owned());
        let label = if member {
            format!("Remove from {}", group.name)
        } else {
            format!("Add to {}", group.name)
        };
        list = list.child(
            button(k, &format!("member-{}", group.name), &label, true)
                .on_press(move |s: &mut AppState| s.model.toggle_member(&name, &id)),
        );
    }
    list.child(new_group_form(s, k))
}

/// Property pane of the selected group: rename, members, visibility and delete.
pub fn properties(s: &AppState, t: &Theme, k: &Kit, name: &str) -> El {
    let Some(group) = s.model.group(name) else {
        return col();
    };
    let members: Vec<&str> = s
        .model
        .imported
        .iter()
        .flat_map(|i| &i.components)
        .filter(|c| group.component_ids.contains(&c.id))
        .map(|c| c.display_name.as_str())
        .collect();
    let hidden = s.model.group_hidden(name);
    let (hide, isolate, delete) = (name.to_owned(), name.to_owned(), name.to_owned());
    let fields = col()
        .w_full()
        .padding(space::S2)
        .gap(space::S1)
        .child(
            row()
                .w_full()
                .gap(space::S1)
                .child(
                    TextInput::new(&s.model.rename_group)
                        .label("Group name")
                        .width(None)
                        .height(22.0)
                        .on_edit(|s: &mut AppState| &mut s.model.rename_group)
                        .build(k)
                        .key("rename-group-name")
                        .grow(1.0),
                )
                .child(
                    button(k, "rename-group", "Rename", true).on_press(|s: &mut AppState| {
                        let result = s.model.rename_selected_group();
                        report(s, result)
                    }),
                ),
        )
        .child(label_value(t, "Bodies", members.len().to_string()))
        .child(label_value(
            t,
            "Members",
            if members.is_empty() {
                "None. Select a body and use Add to group.".to_string()
            } else {
                members.join(", ")
            },
        ))
        .child(
            row()
                .gap(space::S1)
                .child(
                    button(
                        k,
                        "hide-group",
                        if hidden { "Show (V)" } else { "Hide (V)" },
                        !members.is_empty(),
                    )
                    .on_press(move |s: &mut AppState| s.model.toggle_group_hidden(&hide)),
                )
                .child(
                    button(k, "isolate-group", "Isolate (I)", !members.is_empty())
                        .on_press(move |s: &mut AppState| s.model.toggle_group_isolate(&isolate)),
                )
                .child(
                    button(k, "delete-group", "Delete group", true)
                        .on_press(move |s: &mut AppState| s.model.delete_group(&delete)),
                ),
        );
    col()
        .key("group-properties")
        .w_full()
        .child(heading(t, &format!("Group: {name}")))
        .child(fields)
}
