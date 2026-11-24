//! UI-level tests of the shell: model tree, shortcuts, preferences, messages and accessibility.

use rune::{Env, Key, KeyEvent, Mods, Runtime};
use serde_json::Value;

use super::{on_key, view};
use crate::api::dto::Severity;
use crate::format::LengthUnit;
use crate::state::AppState;
use crate::state::model::tests::{body, summary};
use crate::state::preview::ViewportMode;
use crate::state::section::PlaneKind;
use crate::state::shell::Message;

pub(super) fn runtime_with(state: AppState) -> Runtime<AppState> {
    let mut rt = Runtime::new(
        state,
        view,
        (1280.0, 900.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    rt.on_key = Some(on_key);
    rt.frame(0.0);
    rt
}

/// Two bodies in different sub-assemblies.
pub(super) fn motor() -> AppState {
    let mut state = AppState::default();
    let mut s = summary(&[]);
    s.components = vec![
        body("body-1", "Rotor", &["Motor", "Rotor"]),
        body("body-2", "Stator", &["Motor", "Housing", "Stator"]),
    ];
    s.component_count = 2;
    state.model.set_imported(s);
    state
}

pub(super) fn click(rt: &mut Runtime<AppState>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

pub(super) fn press(rt: &mut Runtime<AppState>, key: Key, mods: Mods) {
    rt.key(KeyEvent { key, mods });
    rt.frame(1.0 / 60.0);
}

fn ctrl(c: char) -> (Key, Mods) {
    (
        Key::Command(c),
        Mods {
            cmd: true,
            ctrl: true,
            ..Mods::default()
        },
    )
}

pub(super) fn text_key(t: &str) -> Key {
    Key::Text(t.into())
}

pub(super) fn dump(rt: &Runtime<AppState>) -> String {
    rt.accessibility_json().to_string()
}

#[test]
fn filter_narrows_bodies_reports_the_count_and_nests_by_assembly() {
    let mut rt = runtime_with(motor());
    assert!(dump(&rt).contains("Bodies (2)"));
    // The shared root "Motor" is skipped; "Housing" is a collapsible folder.
    assert!(dump(&rt).contains("Housing, 1 body, expanded"));
    assert!(rt.find("folder-Motor").is_none());
    click(&mut rt, "folder-Motor / Housing");
    assert!(rt.find("body-body-2").is_none() && rt.find("body-body-1").is_some());
    click(&mut rt, "tree-filter");
    press(&mut rt, text_key("stat"), Mods::default());
    assert_eq!(rt.state.model.filter.text, "stat");
    assert!(rt.find("body-body-2").is_some() && rt.find("body-body-1").is_none());
    assert!(dump(&rt).contains("Bodies (1 of 2)"));
    press(&mut rt, text_key("x"), Mods::default());
    assert!(dump(&rt).contains("No bodies match this filter."));
}

#[test]
fn a_group_made_from_the_selected_body_shows_in_the_tree_and_hides_with_v() {
    let mut rt = runtime_with(motor());
    click(&mut rt, "body-body-1");
    rt.frame(0.2); // the pane swaps from Section to Body; let layout settle
    click(&mut rt, "new-group-name");
    press(&mut rt, text_key("Magnets"), Mods::default());
    click(&mut rt, "new-group");
    assert_eq!(rt.state.model.selected_group.as_deref(), Some("Magnets"));
    let d = dump(&rt);
    assert!(d.contains("Groups (1)") && d.contains("Group Magnets"));
    assert!(d.contains("Group Magnets, 1 body"));
    click(&mut rt, "body-body-2");
    click(&mut rt, "member-Magnets");
    assert_eq!(rt.state.model.groups[0].component_ids, ["body-1", "body-2"]);
    click(&mut rt, "group-Magnets");
    press(&mut rt, text_key("v"), Mods::default());
    assert!(rt.state.model.group_hidden("Magnets"));
    click(&mut rt, "delete-group");
    assert!(rt.state.model.groups.is_empty() && rt.find("groups-heading").is_none());
}

#[test]
fn escape_clears_the_filter_and_ctrl_f_focuses_it() {
    let mut rt = runtime_with(motor());
    let (key, mods) = ctrl('f');
    press(&mut rt, key, mods);
    assert!(rt.state.model.filter.focused);
    press(&mut rt, text_key("rot"), Mods::default());
    press(&mut rt, Key::Escape, Mods::default());
    assert!(rt.state.model.filter.text.is_empty());
}

#[test]
fn duplicate_names_show_the_export_name() {
    let mut state = motor();
    let mut s = summary(&[]);
    s.components = vec![body("a", "Bolt", &[]), body("b", "Bolt", &[])];
    s.component_count = 2;
    state.model.set_imported(s);
    let rt = runtime_with(state);
    assert!(dump(&rt).contains("Bolt Bolt_1"));
}

#[test]
fn selecting_a_body_shows_its_source_name_color_and_formatted_size() {
    let mut rt = runtime_with(motor());
    click(&mut rt, "body-body-1");
    let d = dump(&rt);
    for needle in [
        "Body Rotor",
        "Source name",
        "#ff0000",
        "Motor / Rotor",
        "(10.000 \u{d7} 20.000 \u{d7} 30.000)\u{a0}mm",
    ] {
        assert!(d.contains(needle), "missing {needle:?} in properties");
    }
}

#[test]
fn hide_and_isolate_buttons_and_keys_only_change_display_visibility() {
    let mut rt = runtime_with(motor());
    click(&mut rt, "eye-body-1");
    assert!(rt.state.model.hidden.contains("body-1"));
    click(&mut rt, "body-body-2");
    press(&mut rt, text_key("i"), Mods::default());
    assert!(
        rt.state.model.hidden.is_empty(),
        "body-1 was already the only visible-by-isolation set"
    );
    press(&mut rt, text_key("i"), Mods::default());
    assert_eq!(rt.state.model.hidden, ["body-1".to_string()].into());
    press(&mut rt, text_key("v"), Mods::default());
    assert!(rt.state.model.hidden.contains("body-2"));
    press(&mut rt, Key::Escape, Mods::default());
    assert!(rt.state.model.selected.is_none());
}

#[test]
fn global_shortcuts_pick_planes_views_and_undo_redo() {
    let mut rt = runtime_with(motor());
    press(&mut rt, text_key("2"), Mods::default());
    assert_eq!(rt.state.section.plane, PlaneKind::XZ);
    press(&mut rt, text_key("3"), Mods::default());
    assert_eq!(rt.state.section.plane, PlaneKind::YZ);
    // Both plane keys fall inside the coalescing window: one undo step.
    let (key, mods) = ctrl('z');
    press(&mut rt, key.clone(), mods);
    assert_eq!(rt.state.section.plane, PlaneKind::XY);
    press(
        &mut rt,
        key,
        Mods {
            shift: true,
            ..mods
        },
    );
    assert_eq!(rt.state.section.plane, PlaneKind::YZ);
    let (key, mods) = ctrl('2');
    press(&mut rt, key, mods);
    assert_eq!(rt.state.preview.mode, ViewportMode::ThreeD);
    let (key, mods) = ctrl('1');
    press(&mut rt, key, mods);
    assert_eq!(rt.state.preview.mode, ViewportMode::TwoD);
}

#[test]
fn plane_keys_do_nothing_without_a_model_and_never_while_typing() {
    let mut rt = runtime_with(AppState::default());
    press(&mut rt, text_key("2"), Mods::default());
    assert_eq!(rt.state.section.plane, PlaneKind::XY);
    let mut rt = runtime_with(motor());
    click(&mut rt, "tree-filter");
    press(&mut rt, text_key("3"), Mods::default());
    assert_eq!(rt.state.section.plane, PlaneKind::XY);
    assert_eq!(rt.state.model.filter.text, "3");
}

#[test]
fn preferences_change_the_displayed_units_and_restore_defaults() {
    let mut rt = runtime_with(motor());
    click(&mut rt, "body-body-1");
    click(&mut rt, "open-settings");
    click(&mut rt, "unit/segment3");
    assert_eq!(rt.state.shell.number_format.length_unit, LengthUnit::In);
    click(&mut rt, "length-decimals-dec");
    assert_eq!(rt.state.shell.number_format.length_decimals, 2);
    click(&mut rt, "restore-format");
    assert_eq!(rt.state.shell.number_format, Default::default());
    click(&mut rt, "open-settings");
    assert!(!rt.state.shell.show_prefs);
}

#[test]
fn clicking_a_diagnostic_focuses_its_body_and_shows_the_2d_view() {
    let mut state = motor();
    state.preview.mode = ViewportMode::ThreeD;
    state.shell.messages.push(Message {
        severity: Severity::Warning,
        text: "gap".into(),
        component_id: Some("body-2".into()),
        related_component_id: Some("body-1".into()),
    });
    let mut rt = runtime_with(state);
    assert!(rt.find("message-0").is_none(), "collapsed until opened");
    click(&mut rt, "status-warnings");
    click(&mut rt, "message-0");
    assert_eq!(rt.state.model.selected.as_deref(), Some("body-2"));
    assert_eq!(rt.state.model.related.as_deref(), Some("body-1"));
    assert_eq!(rt.state.preview.mode, ViewportMode::TwoD);
}

fn walk<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    out.push(node);
    for child in node["children"].as_array().into_iter().flatten() {
        walk(child, out);
    }
}

#[test]
fn every_interactive_control_has_a_role_and_a_name() {
    let mut rt = runtime_with(motor());
    let tree = rt.accessibility_json();
    let mut nodes = vec![];
    walk(&tree, &mut nodes);
    let named = |role: &str, name: &str| {
        nodes
            .iter()
            .any(|n| n["role"] == role && n["name"].as_str().is_some_and(|v| v.contains(name)))
    };
    for (role, name) in [
        ("toolbar", "Toolbar"),
        ("button", "Open STEP"),
        ("button", "Save"),
        ("button", "Open project"),
        ("radio-button", "XY"),
        ("button", "Compute section"),
        ("button", "Export DXF"),
        ("button", "Undo"),
        ("button", "Redo"),
        ("button", "Settings"),
        ("button", "Rotor"),
        ("region", "Model builder"),
        ("region", "Messages"),
        ("region", "Settings"),
        ("status", "Status"),
    ] {
        assert!(named(role, name), "no {role} named {name:?}");
    }
    assert!(nodes.iter().any(|n| n["name"] == "Filter bodies"));
    click(&mut rt, "open-settings");
    let tree = rt.accessibility_json();
    let mut nodes = vec![];
    walk(&tree, &mut nodes);
    for n in &nodes {
        let role = n["role"].as_str().unwrap_or("");
        if matches!(role, "button" | "check-box" | "radio-button") {
            assert!(
                n["name"].as_str().is_some_and(|v| !v.is_empty()),
                "unnamed {role}: {n}"
            );
        }
    }
    assert!(
        nodes
            .iter()
            .any(|n| n["role"] == "radio-button" && n["name"] == "Dark")
    );
    assert!(nodes.iter().any(|n| n["role"] == "check-box"));
}

#[test]
fn confirmations_raise_a_toast_beside_the_log_and_close_it() {
    let mut rt = runtime_with(motor());
    rt.state
        .shell
        .notify("Project saved", "Project saved to p.c2mproj".into());
    rt.invalidate();
    rt.frame(1.0 / 60.0);
    assert!(dump(&rt).contains("Notifications"));
    let last = rt.state.shell.messages.last().expect("logged");
    assert_eq!(last.text, "Project saved to p.c2mproj");
    click(&mut rt, "close");
    assert!(rt.state.shell.toast.is_none());
    assert_eq!(rt.state.shell.messages.len(), 1, "the log keeps the record");
}

#[test]
fn the_section_node_sits_under_the_model_and_shows_plane_and_offset() {
    let mut rt = runtime_with(motor());
    assert!(rt.find("node-prefs").is_none(), "Preferences left the tree");
    let y = |key: &str| rt.find(key).expect(key).1;
    assert!(y("node-model") < y("node-section") && y("node-section") < y("bodies-heading"));
    // Bounds are 0..30 in z, so the default XY offset is the 15 mm midpoint.
    assert!(dump(&rt).contains("XY, z = 15.000\u{a0}mm"));
    assert!(
        rt.find("section-offset").is_some(),
        "section is selected after import"
    );
    click(&mut rt, "node-model");
    assert!(rt.find("section-offset").is_none());
    click(&mut rt, "node-section");
    assert!(rt.state.model.section_selected && rt.find("section-offset").is_some());
    press(&mut rt, text_key("2"), Mods::default());
    assert!(dump(&rt).contains("XZ, y = 10.000\u{a0}mm"));
}

#[test]
fn tree_rows_have_one_height_and_status_is_text_not_colour() {
    let mut rt = runtime_with(motor());
    let y = |rt: &Runtime<AppState>, key: &str| rt.find(key).expect(key).1;
    let step = y(&rt, "node-section") - y(&rt, "node-model");
    assert_eq!(step, super::tree_row::ROW_H);
    assert_eq!(y(&rt, "bodies-heading") - y(&rt, "node-section"), step);
    click(&mut rt, "eye-body-1");
    assert!(dump(&rt).contains("Show body"));
    assert!(
        dump(&rt).contains("hidden"),
        "hidden rows carry a text label"
    );
    click(&mut rt, "eye-body-1");
    click(&mut rt, "body-body-2");
    press(&mut rt, text_key("i"), Mods::default());
    assert!(rt.state.model.isolated);
    assert!(dump(&rt).contains("isolated"));
    // Chevrons collapse the Bodies node.
    click(&mut rt, "bodies-heading");
    assert!(rt.find("body-body-2").is_none());
}
