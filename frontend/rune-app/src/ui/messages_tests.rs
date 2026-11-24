//! Messages panel: collapsed with only info, auto-open on warnings, severity filter, resize.

use rune::prefs::Prefs;
use rune::{Key, Mods};

use super::shell_tests::{click, dump, motor, press, runtime_with};
use crate::api::dto::Severity;
use crate::state::layout::{Layout, MESSAGES_DEFAULT};
use crate::state::shell::ShellState;

#[test]
fn info_keeps_messages_collapsed_and_a_warning_opens_it() {
    let mut state = motor();
    state.shell.log(Severity::Info, "Imported 2 bodies.");
    let mut rt = runtime_with(state);
    assert!(!rt.state.shell.messages_open && rt.find("message-0").is_none());
    assert!(rt.find("messages-toggle").is_some());
    rt.state.shell.log(Severity::Warning, "Gap of 0.2 mm");
    rt.invalidate();
    rt.frame(1.0 / 60.0);
    assert!(rt.state.shell.messages_open && rt.find("message-1").is_some());
    click(&mut rt, "messages-toggle");
    assert!(
        rt.find("message-1").is_none(),
        "the user can collapse it again"
    );
}

#[test]
fn the_severity_filter_and_status_counts_narrow_the_list() {
    let mut state = motor();
    state.shell.log(Severity::Info, "Imported.");
    state.shell.log(Severity::Warning, "Gap.");
    state.shell.log(Severity::Error, "Overlap.");
    let mut rt = runtime_with(state);
    assert_eq!(
        (
            rt.state.shell.count(Severity::Error),
            rt.state.shell.count(Severity::Warning)
        ),
        (1, 1)
    );
    assert!(dump(&rt).contains("Show errors in Messages"));
    click(&mut rt, "status-errors");
    assert_eq!(rt.state.shell.severity_filter, Some(Severity::Error));
    assert!(rt.find("message-2").is_some() && rt.find("message-1").is_none());
    click(&mut rt, "message-filter/segment0");
    assert!(rt.find("message-0").is_some() && rt.find("message-1").is_some());
}

#[test]
fn dragging_the_top_edge_resizes_and_double_click_resets_and_it_is_remembered() {
    let path = std::env::temp_dir().join(format!("c2m-layout-{}.json", std::process::id()));
    let mut state = motor();
    state.shell = ShellState::with_prefs(Prefs::at(&path));
    state.shell.log(Severity::Warning, "Gap.");
    let mut rt = runtime_with(state);
    let (x, y) = rt.find("split-messages").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_move(x, y - 20.0);
    rt.pointer_move(x, y - 60.0);
    rt.pointer_up(x, y - 60.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(
        rt.state.shell.layout.messages_height,
        MESSAGES_DEFAULT + 60.0
    );
    let again = ShellState::with_prefs(Prefs::at(&path));
    assert_eq!(again.layout.messages_height, MESSAGES_DEFAULT + 60.0);
    let (x, y) = rt.find("split-messages").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.shell.layout, Layout::default());
    press(&mut rt, Key::Escape, Mods::default());
    std::fs::remove_file(path).ok();
}
