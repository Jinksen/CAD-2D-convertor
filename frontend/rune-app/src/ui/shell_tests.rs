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

fn runtime_with(state: AppState) -> Runtime<AppState> {
    let mut rt = Runtime::new(
        state,
        view,
        (1280.0, 760.0),
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
fn motor() -> AppState {
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

fn click(rt: &mut Runtime<AppState>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

fn press(rt: &mut Runtime<AppState>, key: Key, mods: Mods) {
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

fn text_key(t: &str) -> Key {
    Key::Text(t.into())
}

fn dump(rt: &Runtime<AppState>) -> String {
    rt.accessibility_json().to_string()
}

#[test]
fn filter_narrows_bodies_reports_the_count_and_groups_by_assembly() {
    let mut rt = runtime_with(motor());
    assert!(dump(&rt).contains("Bodies (2)"));
    assert!(dump(&rt).contains("Housing (1)") && dump(&rt).contains("Motor (1)"));
    click(&mut rt, "tree-filter");
    press(&mut rt, text_key("stat"), Mods::default());
    assert_eq!(rt.state.model.filter.text, "stat");
    assert!(rt.find("body-body-2").is_some() && rt.find("body-body-1").is_none());
    let d = dump(&rt);
    assert!(d.contains("Bodies (1 of 2)") && d.contains("Housing (1)"));
    assert!(!d.contains("Motor (1)"));
    press(&mut rt, text_key("x"), Mods::default());
    assert!(dump(&rt).contains("No bodies match this filter."));
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
        "Body: Rotor",
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
    click(&mut rt, "node-prefs");
    click(&mut rt, "unit-in");
    assert_eq!(rt.state.shell.number_format.length_unit, LengthUnit::In);
    click(&mut rt, "length-decimals-dec");
    assert_eq!(rt.state.shell.number_format.length_decimals, 2);
    click(&mut rt, "restore-format");
    assert_eq!(rt.state.shell.number_format, Default::default());
    click(&mut rt, "node-prefs");
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
        ("button", "XY"),
        ("button", "Compute section"),
        ("button", "Export DXF"),
        ("button", "Undo"),
        ("button", "Redo"),
        ("button", "Preferences"),
        ("button", "Rotor"),
        ("region", "Model builder"),
        ("region", "Messages"),
        ("region", "Settings"),
        ("status", "Status"),
    ] {
        assert!(named(role, name), "no {role} named {name:?}");
    }
    assert!(nodes.iter().any(|n| n["name"] == "Filter bodies"));
    click(&mut rt, "node-prefs");
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
