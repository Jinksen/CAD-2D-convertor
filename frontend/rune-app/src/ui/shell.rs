//! Toolbar, messages pane and status bar.

use rune::tok::{Theme, space, type_};
use rune::{El, FileDialogOptions, ScrollSpec, Tx, col, row, spacer, text};

use super::{MESSAGES_HEIGHT, button, heading, severity_color, severity_label};
use crate::api::dto::Severity;
use crate::state::AppState;
use crate::state::preview::ViewportMode;
use crate::state::shell::{BackendStatus, Theme as ThemeChoice};

fn dialog(title: &str, file_name: Option<&str>) -> FileDialogOptions {
    FileDialogOptions {
        title: title.into(),
        file_name: file_name.map(Into::into),
        ..Default::default()
    }
}

fn open_step_dialog(_: &mut AppState, tx: &mut Tx) {
    let options = dialog("Open STEP file", None).filter("STEP", &["step", "stp"]);
    tx.open_file(options, |s: &mut AppState, result, _| match result {
        Ok(Some(path)) => s.open_step(path),
        Ok(None) => {}
        Err(error) => s.shell.log(Severity::Error, error.to_string()),
    });
}

fn export_dialog(s: &mut AppState, tx: &mut Tx) {
    if s.section.last.is_none() {
        s.shell
            .log(Severity::Warning, "Compute a section before exporting.");
        return;
    }
    let options = dialog("Save CAD2Maxwell DXF bundle", Some("section.zip"))
        .filter("DXF bundle (ZIP)", &["zip"]);
    tx.save_file(options, |s: &mut AppState, result, _| match result {
        Ok(Some(path)) => s.export_section(path),
        Ok(None) => {}
        Err(error) => s.shell.log(Severity::Error, error.to_string()),
    });
}

pub fn toolbar(s: &AppState, t: &Theme) -> El {
    let idle = s.shell.busy.is_none();
    let has_model = s.model.imported.is_some();
    let has_section = s.section.last.is_some();
    row()
        .key("toolbar")
        .role("toolbar")
        .label("Toolbar")
        .w_full()
        .gap(space::S2)
        .padding(space::S2)
        .items_center()
        .border(1.0, t.separator)
        .child(text("CAD2Maxwell").font(type_::TITLE).color(t.text))
        .child(button(t, "open-step", "Open STEP...", idle).on_press_tx(open_step_dialog))
        .child(
            button(t, "run-section", "Compute section", idle && has_model)
                .on_press(AppState::compute_section),
        )
        .child(
            button(t, "export-dxf", "Export DXF...", idle && has_section)
                .on_press_tx(export_dialog),
        )
        .child(
            button(t, "undo-plane", "Undo", !s.section.past.is_empty())
                .on_press(|s: &mut AppState| s.section.undo()),
        )
        .child(
            button(t, "redo-plane", "Redo", !s.section.future.is_empty())
                .on_press(|s: &mut AppState| s.section.redo()),
        )
        .child(spacer())
        .child(
            button(
                t,
                "toggle-view",
                if s.preview.mode == ViewportMode::TwoD {
                    "View: 2D"
                } else {
                    "View: 3D"
                },
                true,
            )
            .on_press(|s: &mut AppState| {
                s.preview.mode = match s.preview.mode {
                    ViewportMode::TwoD => ViewportMode::ThreeD,
                    ViewportMode::ThreeD => ViewportMode::TwoD,
                }
            }),
        )
        .child(
            button(t, "toggle-theme", "Theme", true).on_press(|s: &mut AppState| {
                s.shell.theme = match s.shell.theme {
                    ThemeChoice::System => ThemeChoice::Light,
                    ThemeChoice::Light => ThemeChoice::Dark,
                    ThemeChoice::Dark => ThemeChoice::System,
                }
            }),
        )
}

pub fn messages(s: &AppState, t: &Theme) -> El {
    let rows = s.shell.messages.iter().enumerate().rev().map(|(index, m)| {
        let focus = m
            .component_id
            .clone()
            .map(|id| (id, m.related_component_id.clone()));
        let line = row()
            .key(format!("message-{index}"))
            .w_full()
            .gap(space::S2)
            .px(space::S2)
            .py(2.0)
            .hover(t.hover)
            .child(
                text(severity_label(m.severity))
                    .font(type_::LABEL)
                    .color(severity_color(m.severity, t))
                    .w(44.0),
            )
            .child(text(&m.text).font(type_::BODY).color(t.text).grow(1.0));
        match focus {
            Some((id, related)) => {
                line.on_press(move |s: &mut AppState| s.focus_component(&id, related.clone()))
            }
            None => line,
        }
    });
    col()
        .key("messages")
        .role("region")
        .label("Messages")
        .w_full()
        .h(MESSAGES_HEIGHT)
        .border(1.0, t.separator)
        .child(heading(
            t,
            &format!("Messages ({})", s.shell.messages.len()),
        ))
        .child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .children(rows),
        )
}

pub fn status_bar(s: &AppState, t: &Theme) -> El {
    let backend = match &s.shell.backend {
        BackendStatus::Connecting => "Geometry service: connecting".to_string(),
        BackendStatus::Online {
            service,
            api_version,
        } => format!("Geometry service: {service} {api_version}"),
        BackendStatus::Offline(reason) => format!("Geometry service: offline ({reason})"),
    };
    let backend_color = if matches!(s.shell.backend, BackendStatus::Offline(_)) {
        super::severity_color(Severity::Error, t)
    } else {
        t.text_secondary
    };
    row()
        .key("status")
        .role("status")
        .label("Status")
        .w_full()
        .gap(space::S3)
        .px(space::S2)
        .py(space::S1)
        .child(text(backend).font(type_::LABEL).color(backend_color))
        .child(text("Units: mm").font(type_::LABEL).color(t.text_secondary))
        .child(spacer())
        .child(
            text(
                s.shell
                    .busy
                    .as_deref()
                    .map_or_else(String::new, |b| format!("{b}...")),
            )
            .font(type_::LABEL)
            .color(t.text),
        )
        .child(
            text(s.section.stats.map_or_else(String::new, |st| {
                format!(
                    "Regions {}  Errors {}  Warnings {}",
                    st.regions, st.errors, st.warnings
                )
            }))
            .font(type_::LABEL)
            .color(t.text_secondary),
        )
}
