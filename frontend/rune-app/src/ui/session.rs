//! Unsaved-changes prompt and the recent-projects list (dialog and welcome-screen block).

use rune::tok::{Theme, space, type_};
use rune::{El, Tx, col, row, text};
use rune_kit::{Dialog, Kit};

use super::button;
use crate::state::AppState;

/// Recent projects as rows: name, folder, a `not found` flag for missing files, Open and Remove.
pub fn recent_list(s: &AppState, t: &Theme, k: &Kit) -> El {
    let shown = s
        .shell
        .recent
        .shown(s.shell.settings.general.recent_count)
        .to_vec();
    let mut list = col()
        .key("recent-list")
        .role("group")
        .label("Recent projects")
        .w_full()
        .gap(space::S1);
    if shown.is_empty() {
        return list.child(
            text("No recent projects.")
                .font(type_::BODY)
                .color(t.text_secondary),
        );
    }
    for (i, path) in shown.into_iter().enumerate() {
        let exists = path.is_file();
        let name = path.file_stem().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let (open, remove) = (path.clone(), path.clone());
        let mut info = col()
            .grow(1.0)
            .min_w(0.0)
            .child(text(name).font(type_::BODY).color(t.text));
        info = info.child(
            text(if exists {
                path.display().to_string()
            } else {
                format!("Not found: {}", path.display())
            })
            .font(type_::LABEL)
            .color(t.text_secondary),
        );
        list = list.child(
            row()
                .w_full()
                .gap(space::S1)
                .items_center()
                .child(info)
                .child(
                    button(k, &format!("recent-open-{i}"), "Open", exists).on_press_tx(
                        move |s: &mut AppState, tx: &mut Tx| s.open_recent(open.clone(), tx),
                    ),
                )
                .child(
                    button(k, &format!("recent-remove-{i}"), "Remove", true)
                        .on_press(move |s: &mut AppState| s.shell.remove_recent(&remove)),
                ),
        );
    }
    list
}

/// The recent-projects dialog and the unsaved-changes prompt as overlay layers.
pub fn overlay(s: &AppState, t: &Theme, k: &Kit) -> Vec<El> {
    let mut layer = vec![];
    Dialog::new("recent", "Recent projects")
        .open(s.shell.recent_open && s.shell.pending.is_none())
        .width(560.0)
        .on_close(|s: &mut AppState| s.shell.recent_open = false)
        .child(
            col()
                .w_full()
                .padding(space::S2)
                .child(recent_list(s, t, k)),
        )
        .build(k, &mut layer);
    let body = col()
        .w_full()
        .padding(space::S3)
        .gap(space::S2)
        .child(
            text("The project has unsaved changes. Save them before continuing?")
                .font(type_::BODY)
                .color(t.text)
                .wrap(),
        )
        .child(
            row()
                .gap(space::S1)
                .child(
                    button(k, "unsaved-save", "Save", true)
                        .on_press_tx(|s: &mut AppState, tx: &mut Tx| s.save_and_continue(tx)),
                )
                .child(
                    button(k, "unsaved-discard", "Discard changes", true)
                        .on_press_tx(|s: &mut AppState, tx: &mut Tx| s.discard_and_continue(tx)),
                )
                .child(
                    button(k, "unsaved-cancel", "Cancel", true)
                        .on_press(|s: &mut AppState| s.shell.pending = None),
                ),
        );
    Dialog::new("unsaved", "Unsaved changes")
        .open(s.shell.pending.is_some())
        .width(480.0)
        .fit()
        .on_close(|s: &mut AppState| s.shell.pending = None)
        .child(body)
        .build(k, &mut layer);
    layer
}
