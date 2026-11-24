//! File dialogs, toasts and global keyboard shortcuts.

use rune::{El, FileDialogOptions, Key, KeyEvent, Tx};
use rune_kit::{Kit, Toast, ToastRegion};

use crate::api::dto::Severity;
use crate::state::preview::ViewportMode;
use crate::state::section::PlaneKind;
use crate::state::{AppState, now_ms};

const TOAST_TTL: std::time::Duration = std::time::Duration::from_secs(5);

fn dialog(title: &str, file_name: Option<&str>) -> FileDialogOptions {
    FileDialogOptions {
        title: title.into(),
        file_name: file_name.map(Into::into),
        ..Default::default()
    }
}

pub fn open_step_dialog(_: &mut AppState, tx: &mut Tx) {
    let options = dialog("Open model", None).filter("CAD files", &crate::tasks::MODEL_EXTENSIONS);
    tx.open_file(options, |s: &mut AppState, result, _| match result {
        Ok(Some(path)) => s.open_step(path),
        Ok(None) => {}
        Err(error) => s.shell.log(Severity::Error, error.to_string()),
    });
}

pub(super) fn open_project_dialog(_: &mut AppState, tx: &mut Tx) {
    let options = dialog("Open project", None).filter("CAD2Maxwell project", &["c2mproj"]);
    tx.open_file(options, |s: &mut AppState, result, _| match result {
        Ok(Some(path)) => s.open_project(path),
        Ok(None) => {}
        Err(error) => s.shell.log(Severity::Error, error.to_string()),
    });
}

/// Saves to the known project path, else (or with `save_as`) asks for one.
pub(super) fn save_project_dialog(s: &mut AppState, tx: &mut Tx, save_as: bool) {
    if s.model.imported.is_none() {
        s.shell
            .log(Severity::Error, "Open a model before saving a project.");
    } else if let (false, Some(path)) = (save_as, s.shell.project_path.clone()) {
        s.save_project(path);
    } else {
        let options = dialog("Save project", Some("project.c2mproj"))
            .filter("CAD2Maxwell project", &["c2mproj"]);
        tx.save_file(options, |s: &mut AppState, result, _| match result {
            Ok(Some(path)) => s.save_project(path),
            Ok(None) => {}
            Err(error) => s.shell.log(Severity::Error, error.to_string()),
        });
    }
}

pub(super) fn exportable(s: &AppState) -> bool {
    s.section.export_blocker().is_none()
}

pub(super) fn export_dialog(s: &mut AppState, tx: &mut Tx) {
    if let Some(reason) = s.section.export_blocker() {
        s.shell.log(Severity::Warning, reason);
        return;
    }
    let settings = &s.shell.settings.export;
    let mut options = if settings.zip {
        dialog("Save CAD2Maxwell DXF bundle", Some("section.zip"))
            .filter("DXF bundle (ZIP)", &["zip"])
    } else {
        dialog("Name the export folder", Some("section"))
    };
    options.directory = settings
        .folder
        .as_ref()
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_dir());
    tx.save_file(options, |s: &mut AppState, result, _| match result {
        Ok(Some(path)) => s.export_section(path),
        Ok(None) => {}
        Err(error) => s.shell.log(Severity::Error, error.to_string()),
    });
}

pub(super) fn compute(s: &mut AppState) {
    s.preview.mode = ViewportMode::TwoD;
    s.compute_section();
}

/// The newest confirmation (save, export) as a toast; Messages stays the record.
pub fn toasts(s: &AppState, k: &Kit) -> Vec<El> {
    let Some((title, detail, _)) = &s.shell.toast else {
        return vec![];
    };
    let toast = Toast::new("toast", *title)
        .description(detail.clone())
        .on_close(|s: &mut AppState| s.shell.toast = None);
    vec![ToastRegion::new().toast(toast.build(k)).build(k)]
}

/// Window-level shortcuts of the React app. Keys typed into a focused field never get here.
pub fn on_key(s: &mut AppState, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.mods.alt {
        return false;
    }
    let idle = s.shell.busy.is_none();
    let has_model = s.model.imported.is_some();
    if ev.mods.cmd {
        match &ev.key {
            Key::Command('o') => {
                if idle {
                    open_step_dialog(s, tx)
                }
            }
            Key::Enter => {
                if idle && has_model {
                    compute(s)
                }
            }
            Key::Command('e') => {
                if idle && exportable(s) {
                    export_dialog(s, tx)
                }
            }
            Key::Command(',') => s.shell.show_settings = true,
            Key::Command('s') => save_project_dialog(s, tx, ev.mods.shift),
            Key::Command('z') if !ev.mods.shift => s.section.undo(),
            Key::Command('z' | 'y') => s.section.redo(),
            Key::Command('f') if has_model => s.model.filter.focused = true,
            Key::Command('1') if has_model => s.preview.mode = ViewportMode::TwoD,
            Key::Command('2') if has_model => s.preview.mode = ViewportMode::ThreeD,
            _ => return false,
        }
        return true;
    }
    let selected = s.model.selected.clone();
    let group = s.model.selected_group.clone();
    match &ev.key {
        Key::Text(k) if has_model && matches!(k.as_str(), "1" | "2" | "3") => {
            let kind =
                [PlaneKind::XY, PlaneKind::XZ, PlaneKind::YZ][usize::from(k.as_bytes()[0] - b'1')];
            s.section.set_plane(kind, now_ms());
        }
        Key::Enter if s.model.renaming => {
            if let Err(error) = s.model.commit_rename() {
                s.shell.log(Severity::Error, error);
            }
        }
        Key::Escape if s.model.renaming => s.model.renaming = false,
        Key::Escape => s.model.select(None),
        Key::Up if has_model => s.model.move_cursor(-1),
        Key::Down if has_model => s.model.move_cursor(1),
        Key::Left if has_model => s.model.collapse_or_parent(),
        Key::Right if has_model => s.model.expand_or_child(),
        // Enter opens the properties of the row under the cursor.
        Key::Enter if has_model => {
            let node = s.model.current();
            s.model.activate(&node)
        }
        Key::Function(2) if group.is_some() => s.model.start_rename(&group.unwrap_or_default()),
        Key::Text(k) if selected.is_some() && k.eq_ignore_ascii_case("v") => {
            s.model.toggle_hidden_many(&s.model.targets())
        }
        Key::Text(k) if selected.is_some() && k.eq_ignore_ascii_case("i") => {
            let ids = s.model.targets();
            s.model.isolate(&ids)
        }
        Key::Text(k) if group.is_some() && k.eq_ignore_ascii_case("v") => {
            s.model.toggle_group_hidden(&group.unwrap_or_default())
        }
        Key::Text(k) if group.is_some() && k.eq_ignore_ascii_case("i") => {
            s.model.toggle_group_isolate(&group.unwrap_or_default())
        }
        _ => return false,
    }
    true
}

/// While an operation runs, redraw once a second so its elapsed time counts up; toasts expire.
pub fn poll(s: &mut AppState, _: f64, tx: &mut Tx) -> bool {
    let mut changed = s.shell.busy.is_some() | s.ensure_regions();
    if changed {
        tx.wake_after(std::time::Duration::from_secs(1));
    }
    if let Some((.., at)) = &s.shell.toast {
        match TOAST_TTL.checked_sub(at.elapsed()) {
            Some(left) => tx.wake_after(left),
            None => (s.shell.toast, changed) = (None, true),
        }
    }
    changed
}
