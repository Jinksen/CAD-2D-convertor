//! Welcome screen shown in the viewport while no model is open (spec 13.1): open a model or
//! project, recent projects, the demo motor, documentation and a drop hint.

use std::path::{Path, PathBuf};

use rune::tok::{Theme, space, type_};
use rune::{El, Tx, col, text};
use rune_kit::Kit;

use super::session::recent_list;
use super::shell::{open_project_dialog, open_step_dialog};
use super::{button, rule};
use crate::session::Pending;
use crate::state::AppState;

pub const DOCS_URL: &str = "https://jinksen.github.io/CAD-2D-convertor/";
const DEMO: &str = "docs/demo/demo-motor.step";

/// The demo model: `docs/demo` in a repository checkout above `start`, or `demo` next to it in
/// an installed build.
pub fn find_demo_from(start: &Path) -> Option<PathBuf> {
    start.ancestors().take(8).find_map(|dir| {
        [dir.join(DEMO), dir.join("demo/demo-motor.step")]
            .into_iter()
            .find(|p| p.is_file())
    })
}

pub fn find_demo() -> Option<PathBuf> {
    find_demo_from(std::env::current_exe().ok()?.parent()?)
        .or_else(|| find_demo_from(&std::env::current_dir().ok()?))
}

pub fn welcome(s: &AppState, t: &Theme, k: &Kit) -> El {
    let idle = s.shell.busy.is_none();
    let demo = find_demo();
    let mut demo_button = button(k, "welcome-demo", "Open demo motor", idle && demo.is_some());
    if let Some(path) = demo {
        demo_button = demo_button.on_press_tx(move |s: &mut AppState, tx: &mut Tx| {
            s.guard(Pending::OpenModel(path.clone()), tx)
        });
    }
    let note = |msg: &str| {
        text(msg.to_owned())
            .font(type_::LABEL)
            .color(t.text_secondary)
            .wrap()
    };
    col()
        .key("welcome")
        .role("region")
        .label("Welcome")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .items_center()
        .justify_center()
        .child(
            col()
                .w(520.0)
                .max_w(520.0)
                .gap(space::S2)
                .child(text("CAD2Maxwell").font(type_::TITLE).color(t.text))
                .child(note(
                    "Exact 2D sections of CAD models for ANSYS Maxwell 2D.",
                ))
                .child(
                    rune::row()
                        .gap(space::S1)
                        .child(
                            button(k, "welcome-open-model", "Open model…", idle)
                                .on_press_tx(open_step_dialog),
                        )
                        .child(
                            button(k, "welcome-open-project", "Open project…", idle)
                                .on_press_tx(open_project_dialog),
                        )
                        .child(demo_button),
                )
                .child(if find_demo().is_none() {
                    note("The demo motor was not found next to the application or in a repository checkout (docs/demo/demo-motor.step).")
                } else {
                    note("The demo motor is a small synthetic stator and rotor.")
                })
                .child(rule(t))
                .child(text("Recent projects").font(type_::SECTION).color(t.text_secondary))
                .child(recent_list(s, t, k))
                .child(rule(t))
                .child(
                    rune::row()
                        .gap(space::S1)
                        .items_center()
                        .child(
                            button(k, "welcome-docs", "Documentation", true).on_press_tx(
                                |s: &mut AppState, tx: &mut Tx| {
                                    if let Err(error) = tx.open_url(DOCS_URL) {
                                        s.shell.log(crate::api::dto::Severity::Error, error.to_string());
                                    }
                                },
                            ),
                        )
                        .child(note(DOCS_URL)),
                )
                .child(note(
                    "Drop a STEP, IGES, BREP or .c2mproj file anywhere in this window to open it.",
                )),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_demo_is_found_in_a_checkout_above_the_start_and_absent_otherwise() {
        let root = std::env::temp_dir().join(format!("c2m-demo-{}", std::process::id()));
        let demo = root.join("docs/demo");
        std::fs::create_dir_all(&demo).unwrap();
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        assert_eq!(find_demo_from(&root.join("a/b")), None);
        std::fs::write(demo.join("demo-motor.step"), "x").unwrap();
        assert_eq!(
            find_demo_from(&root.join("a/b")),
            Some(demo.join("demo-motor.step"))
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
