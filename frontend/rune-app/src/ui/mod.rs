//! COMSOL-like layout: toolbar on top, model tree left, viewport centre, settings right,
//! messages below, status bar at the bottom. Each region lives in its own module.

mod model_tree;
mod preview;
mod section;
mod section_view;
mod shell;

use rune::tok::{self, Theme, space, type_};
use rune::{Color, Cx, El, col, el, oklch, row, text};

use crate::api::dto::Severity;
use crate::state::AppState;

pub const TREE_WIDTH: f32 = 260.0;
pub const SETTINGS_WIDTH: f32 = 300.0;
pub const MESSAGES_HEIGHT: f32 = 150.0;

pub fn view(s: &AppState, cx: &mut Cx<AppState>) -> El {
    let t = tok::theme(s.shell.theme.is_dark(cx.system_dark()));
    let accent = cx.accent();
    col()
        .key("root")
        .w_full()
        .h_full()
        .fill(t.bg)
        .child(shell::toolbar(s, &t))
        .child(
            row()
                .key("workspace")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(model_tree::tree(s, &t, accent))
                .child(preview::viewport(s, &t))
                .child(
                    col()
                        .key("properties")
                        .role("region")
                        .label("Settings")
                        .w(SETTINGS_WIDTH)
                        .h_full()
                        .min_h(0.0)
                        .border(1.0, t.separator)
                        .child(section::settings(s, &t, accent))
                        .child(model_tree::properties(s, &t)),
                ),
        )
        .child(shell::messages(s, &t))
        .child(shell::status_bar(s, &t))
        .on_drop_files(|s: &mut AppState, files, _| {
            if let Some(path) = files.first() {
                s.open_step(path.clone());
            }
        })
}

/// Region title bar.
pub fn heading(t: &Theme, title: &str) -> El {
    row()
        .w_full()
        .px(space::S2)
        .py(space::S1)
        .fill(t.control_track)
        .child(text(title).font(type_::SECTION).color(t.text_secondary))
}

/// Compact toolbar / form button; the caller attaches `on_press`.
pub fn button(t: &Theme, key: &str, label: &str, enabled: bool) -> El {
    row()
        .key(key)
        .role("button")
        .label(label)
        .focusable(enabled)
        .disabled(!enabled)
        .px(space::S2)
        .py(space::S1)
        .radius(tok::radius::SM)
        .fill(t.control_off)
        .hover(t.hover)
        .child(text(label).font(type_::LABEL).color(if enabled {
            t.text
        } else {
            t.text_secondary
        }))
}

/// A button that shows it is the active choice of its group.
pub fn toggle(t: &Theme, accent: Color, key: &str, label: &str, on: bool) -> El {
    button(t, key, label, true)
        .fill(if on {
            accent.alpha(0.35)
        } else {
            t.control_off
        })
        .checked(on)
}

pub fn severity_color(severity: Severity, t: &Theme) -> Color {
    match severity {
        Severity::Error => oklch(0.66, 0.19, 25.0),
        Severity::Warning => oklch(0.78, 0.15, 80.0),
        Severity::Info => t.text_secondary,
    }
}

pub fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "ERROR",
        Severity::Warning => "WARN",
        Severity::Info => "INFO",
    }
}

pub fn label_value(t: &Theme, label: &str, value: impl Into<String>) -> El {
    row()
        .w_full()
        .gap(space::S2)
        .child(
            text(label)
                .font(type_::LABEL)
                .color(t.text_secondary)
                .w(96.0),
        )
        .child(
            text(value)
                .font(type_::BODY)
                .color(t.text)
                .truncate()
                .grow(1.0),
        )
}

/// Thin separator line.
pub fn rule(t: &Theme) -> El {
    el().w_full().h(1.0).fill(t.separator)
}

#[cfg(test)]
mod tests {
    use rune::{Env, Runtime};

    use super::*;

    fn runtime() -> Runtime<AppState> {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1280.0, 760.0),
            Env {
                system_dark: true,
                scale: 1.0,
            },
        );
        runtime.frame(0.0);
        runtime
    }

    #[test]
    fn layout_regions_exist_in_the_native_view_tree() {
        let runtime = runtime();
        for key in [
            "toolbar",
            "workspace",
            "model-tree",
            "viewport",
            "properties",
            "messages",
            "status",
        ] {
            assert!(runtime.find(key).is_some(), "missing region {key}");
        }
    }

    #[test]
    fn dropping_a_step_file_starts_an_import_and_other_files_are_refused() {
        let mut runtime = runtime();
        let at = runtime.find("viewport");
        runtime.drop_files(vec!["part.dxf".into()], at);
        runtime.frame(1.0 / 60.0);
        let last = runtime.state.shell.messages.last().expect("a message");
        assert_eq!(last.text, "Choose a STEP or STP file.");
    }

    #[test]
    fn imported_bodies_are_listed_and_selectable() {
        let mut state = AppState::default();
        state
            .model
            .set_imported(crate::state::model::tests::summary(&["a", "b"]));
        let mut runtime = Runtime::new(
            state,
            view,
            (1280.0, 760.0),
            Env {
                system_dark: false,
                scale: 1.0,
            },
        );
        runtime.frame(0.0);
        let (x, y) = runtime.find("body-b").expect("body row");
        runtime.pointer_down(x, y);
        runtime.pointer_up(x, y);
        runtime.frame(1.0 / 60.0);
        assert_eq!(runtime.state.model.selected.as_deref(), Some("b"));
    }
}
