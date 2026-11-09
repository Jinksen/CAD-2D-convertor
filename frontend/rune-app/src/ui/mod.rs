//! COMSOL-like layout: toolbar on top, model tree left, viewport centre, settings right,
//! messages below, status bar at the bottom. Each region lives in its own module.

mod groups;
mod model_tree;
mod preview;
mod section;
mod section_view;
mod shell;

pub use shell::{on_key, poll};

use rune::tok::{self, Theme, space, type_};
use rune::{Color, Cx, El, ScrollSpec, col, el, oklch, row, text};
use rune_kit::{Button, ControlSize, Kit, PaletteSeed, SegmentedControl, scroll_bar};

use crate::api::dto::Severity;
use crate::state::AppState;

pub const TREE_WIDTH: f32 = 260.0;
pub const SETTINGS_WIDTH: f32 = 320.0;
pub const MESSAGES_HEIGHT: f32 = 150.0;

/// Kit tokens derived from the app's neutral theme, with small radii.
pub(crate) fn kit(t: &Theme, accent: Color) -> Kit {
    let mut k = Kit::from_palette(PaletteSeed {
        bg: t.bg,
        fg: t.text,
        accent,
    });
    (k.radius.sm, k.radius.md) = (2.0, 3.0);
    k
}

pub fn view(s: &AppState, cx: &mut Cx<AppState>) -> El {
    let t = tok::theme(s.shell.theme.is_dark(cx.system_dark()));
    let k = kit(&t, cx.accent());
    s.preview.scale.set(cx.env().scale);
    let base = col()
        .key("root")
        .w_full()
        .h_full()
        .fill(t.bg)
        .child(shell::toolbar(s, &t, &k))
        .child(
            row()
                .key("workspace")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(model_tree::tree(s, &t, &k))
                .child(preview::viewport(s, &t, &k))
                .child(
                    col()
                        .key("properties")
                        .role("region")
                        .label("Settings")
                        .w(SETTINGS_WIDTH)
                        .h_full()
                        .min_h(0.0)
                        .border(1.0, t.separator)
                        .child(scroller(
                            &k,
                            "properties-scroll",
                            col().w_full().children(if s.shell.show_prefs {
                                vec![shell::preferences(s, &t, &k)]
                            } else {
                                vec![
                                    section::settings(s, &t, &k),
                                    model_tree::properties(s, &t, &k),
                                ]
                            }),
                        )),
                ),
        )
        .child(shell::messages(s, &t, &k))
        .child(shell::status_bar(s, &t))
        .on_drop_files(|s: &mut AppState, files, _| match files.first() {
            Some(path)
                if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("c2mproj")) =>
            {
                s.open_project(path.clone())
            }
            Some(path) => s.open_step(path.clone()),
            None => {}
        });
    rune_kit::root(base, shell::toasts(s, &k))
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
pub fn button(k: &Kit, key: &str, label: &str, enabled: bool) -> El {
    Button::new(label)
        .control_size(ControlSize::Small)
        .disabled(!enabled)
        .build(k)
        .key(key)
}

/// A compact radio group of equal segments; `pick` gets the chosen index.
pub fn choice<S: AsRef<str>>(
    k: &Kit,
    key: &str,
    label: &str,
    labels: &[S],
    on: Option<usize>,
    pick: impl Fn(&mut AppState, usize) + Clone + 'static,
) -> El {
    SegmentedControl::new(labels, on)
        .label(label)
        .fit()
        .on_pick(move |s: &mut AppState, i, _| pick(s, i))
        .build(k)
        .key(key)
}

/// Fill of a selected tree row.
pub fn selected(k: &Kit) -> Color {
    k.alpha(k.c.text_accent, if k.dark { 0.24 } else { 0.2 })
}

/// A vertical scroll area filling its parent, with the kit's scroll bar.
pub fn scroller(k: &Kit, key: &str, content: El) -> El {
    let viewport = col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(content);
    scroll_bar(k, &format!("{key}-area"), key, viewport)
        .w_full()
        .grow(1.0)
        .min_h(0.0)
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
        Severity::Error => "Error",
        Severity::Warning => "Warning",
        Severity::Info => "Info",
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

#[cfg(test)]
mod shell_tests;
