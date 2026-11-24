//! Resizable and collapsible side panels, persisted in the prefs file.

use rune::prefs::Prefs;

use super::shell_tests::{click, motor, runtime_with};
use crate::state::layout::{Layout, PROPS_DEFAULT, TREE_DEFAULT};
use crate::state::shell::ShellState;

fn drag_by(rt: &mut rune::Runtime<crate::state::AppState>, key: &str, dx: f32) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no {key}"));
    rt.pointer_down(x, y);
    rt.pointer_move(x + dx / 2.0, y);
    rt.pointer_move(x + dx, y);
    rt.pointer_up(x + dx, y);
    rt.frame(1.0 / 60.0);
}

#[test]
fn splitters_resize_the_side_panels_double_click_resets_and_prefs_remember_it() {
    let path = std::env::temp_dir().join(format!("c2m-panels-{}.json", std::process::id()));
    let mut state = motor();
    state.shell = ShellState::with_prefs(Prefs::at(&path));
    let mut rt = runtime_with(state);
    drag_by(&mut rt, "split-tree", 40.0);
    assert_eq!(rt.state.shell.layout.tree_width, TREE_DEFAULT + 40.0);
    drag_by(&mut rt, "split-properties", -50.0);
    assert_eq!(rt.state.shell.layout.props_width, PROPS_DEFAULT + 50.0);
    let saved = ShellState::with_prefs(Prefs::at(&path)).layout;
    assert_eq!(saved, rt.state.shell.layout);
    // Out-of-range drags clamp.
    drag_by(&mut rt, "split-tree", -2000.0);
    assert_eq!(
        rt.state.shell.layout.tree_width,
        crate::state::layout::TREE_RANGE.0
    );
    for key in ["split-tree", "split-properties"] {
        let (x, y) = rt.find(key).unwrap();
        for _ in 0..2 {
            rt.pointer_down(x, y);
            rt.pointer_up(x, y);
        }
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(rt.state.shell.layout, Layout::default());
    std::fs::remove_file(path).ok();
}

#[test]
fn side_panels_collapse_to_a_strip_and_expand_again() {
    let mut rt = runtime_with(motor());
    click(&mut rt, "tree-collapse");
    assert!(rt.state.shell.layout.tree_collapsed);
    assert!(rt.find("body-body-1").is_none() && rt.find("split-tree").is_none());
    click(&mut rt, "properties-collapse");
    assert!(rt.find("section-settings").is_none());
    click(&mut rt, "tree-expand");
    click(&mut rt, "properties-expand");
    let l = rt.state.shell.layout;
    assert!(!l.tree_collapsed && !l.props_collapsed);
    assert!(rt.find("body-body-1").is_some() && rt.find("section-settings").is_some());
}
