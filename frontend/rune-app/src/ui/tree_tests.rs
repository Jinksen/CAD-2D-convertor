//! Model-tree interactions: multi-select, context menu, rename, drag to group, arrow keys.

use rune::{Key, Mods, PhysicalInput, Runtime};

use super::shell_tests::{click, dump, motor, press, runtime_with, text_key};
use crate::state::AppState;
use crate::state::model::BodyGroup;

fn hold_ctrl(rt: &mut Runtime<AppState>, on: bool) {
    rt.physical_input(PhysicalInput::Modifiers(Mods {
        ctrl: on,
        cmd: on,
        ..Mods::default()
    }));
}

fn with_group() -> Runtime<AppState> {
    let mut state = motor();
    state.model.groups.push(BodyGroup {
        name: "North".into(),
        component_ids: vec![],
    });
    runtime_with(state)
}

fn right_click(rt: &mut Runtime<AppState>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no {key}"));
    rt.secondary_down(x, y);
    rt.secondary_up(x, y);
    rt.frame(0.0);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}

#[test]
fn ctrl_click_multi_selects_and_v_hides_all_picked_bodies() {
    let mut rt = runtime_with(motor());
    click(&mut rt, "body-body-1");
    hold_ctrl(&mut rt, true);
    click(&mut rt, "body-body-2");
    hold_ctrl(&mut rt, false);
    assert_eq!(rt.state.model.targets(), ["body-1", "body-2"]);
    assert_eq!(rt.state.model.selected.as_deref(), Some("body-2"));
    press(&mut rt, text_key("v"), Mods::default());
    assert_eq!(rt.state.model.hidden.len(), 2);
}

#[test]
fn right_click_opens_a_per_node_menu_and_hide_applies() {
    let mut rt = with_group();
    right_click(&mut rt, "body-body-1");
    assert!(rt.find("tree-menu-menu").is_some());
    let d = dump(&rt);
    for needle in ["Hide", "Isolate", "Zoom to", "Add to group", "Properties"] {
        assert!(d.contains(needle), "menu lacks {needle}");
    }
    click(&mut rt, "tree-menu-item0");
    assert!(rt.state.model.hidden.contains("body-1"));
    assert!(rt.state.model.menu.is_none());

    right_click(&mut rt, "body-body-2");
    click(&mut rt, "tree-menu-item3"); // Add to group >
    click(&mut rt, "tree-menu-item3-sub-item0");
    assert_eq!(rt.state.model.groups[0].component_ids, ["body-2"]);

    right_click(&mut rt, "group-North");
    assert!(dump(&rt).contains("Rename") && dump(&rt).contains("Delete group"));
}

#[test]
fn f2_renames_a_group_in_place_and_enter_commits() {
    let mut rt = with_group();
    click(&mut rt, "group-North");
    press(&mut rt, Key::Function(2), Mods::default());
    assert!(rt.state.model.renaming && rt.find("rename-inline").is_some());
    rt.state.model.rename_group = rune::TextEdit::new("South");
    rt.invalidate();
    rt.frame(1.0 / 60.0);
    click(&mut rt, "rename-inline");
    press(&mut rt, Key::Enter, Mods::default());
    assert_eq!(rt.state.model.groups[0].name, "South");
    assert!(!rt.state.model.renaming);
}

#[test]
fn dragging_a_body_row_onto_a_group_adds_it() {
    let mut rt = with_group();
    let (x, y) = rt.find("body-body-1").unwrap();
    let (_, gy) = rt.find("group-North").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_move(x, (y + gy) / 2.0);
    rt.pointer_move(x, gy);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.model.drop_target.as_deref(), Some("North"));
    rt.pointer_up(x, gy);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.model.groups[0].component_ids, ["body-1"]);
    assert!(rt.state.model.drop_target.is_none());
}

#[test]
fn arrow_keys_walk_the_tree_and_collapse_or_expand() {
    let mut rt = runtime_with(motor());
    press(&mut rt, Key::Down, Mods::default()); // Section -> Bodies header
    press(&mut rt, Key::Down, Mods::default()); // first body
    assert_eq!(rt.state.model.selected.as_deref(), Some("body-1"));
    press(&mut rt, Key::Up, Mods::default());
    press(&mut rt, Key::Left, Mods::default()); // Bodies collapses
    assert!(rt.find("body-body-1").is_none());
    press(&mut rt, Key::Right, Mods::default());
    assert!(rt.find("body-body-1").is_some());
    press(&mut rt, Key::Enter, Mods::default());
    assert!(rt.find("body-properties").is_some() || rt.find("section-settings").is_none());
}
