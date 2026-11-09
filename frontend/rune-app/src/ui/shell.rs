//! Toolbar, preferences, messages pane, status bar and global keyboard shortcuts.

use rune::tok::{Theme, space, type_};
use rune::{El, FileDialogOptions, Key, KeyEvent, Tx, col, row, spacer, text};
use rune_kit::{Checkbox, Kit, SegmentedControl, State, Toast, ToastRegion};

use super::{
    MESSAGES_HEIGHT, button, choice, heading, label_value, rule, scroller, severity_color,
    severity_label,
};
use crate::api::dto::Severity;
use crate::format::{DecimalMarker, LengthUnit, MAX_DECIMALS, NumberFormat};
use crate::state::preview::ViewportMode;
use crate::state::section::PlaneKind;
use crate::state::shell::{BackendStatus, Theme as ThemeChoice};
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
    let options = dialog("Open STEP file", None).filter("STEP", &["step", "stp"]);
    tx.open_file(options, |s: &mut AppState, result, _| match result {
        Ok(Some(path)) => s.open_step(path),
        Ok(None) => {}
        Err(error) => s.shell.log(Severity::Error, error.to_string()),
    });
}

fn open_project_dialog(_: &mut AppState, tx: &mut Tx) {
    let options = dialog("Open project", None).filter("CAD2Maxwell project", &["c2mproj"]);
    tx.open_file(options, |s: &mut AppState, result, _| match result {
        Ok(Some(path)) => s.open_project(path),
        Ok(None) => {}
        Err(error) => s.shell.log(Severity::Error, error.to_string()),
    });
}

/// Saves to the known project path, else (or with `save_as`) asks for one.
fn save_project_dialog(s: &mut AppState, tx: &mut Tx, save_as: bool) {
    if s.model.imported.is_none() {
        s.shell
            .log(Severity::Error, "Open a STEP file before saving a project.");
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

fn exportable(s: &AppState) -> bool {
    s.section.export_blocker().is_none()
}

fn export_dialog(s: &mut AppState, tx: &mut Tx) {
    if let Some(reason) = s.section.export_blocker() {
        s.shell.log(Severity::Warning, reason);
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

fn compute(s: &mut AppState) {
    s.preview.mode = ViewportMode::TwoD;
    s.compute_section();
}

fn group(label: &str) -> El {
    row()
        .key(format!("group-{label}"))
        .role("group")
        .label(label)
        .gap(space::S1)
        .items_center()
}

const PRINCIPAL: [PlaneKind; 3] = [PlaneKind::XY, PlaneKind::XZ, PlaneKind::YZ];

pub fn toolbar(s: &AppState, t: &Theme, k: &Kit) -> El {
    let idle = s.shell.busy.is_none();
    let has_model = s.model.imported.is_some();
    let names = PRINCIPAL.map(crate::project::kind_name);
    let on = PRINCIPAL.iter().position(|p| *p == s.section.plane);
    let mut planes = SegmentedControl::new(&names, on.filter(|_| has_model))
        .label("Section plane")
        .fit()
        .on_pick(|s: &mut AppState, i, _| s.section.set_plane(PRINCIPAL[i], now_ms()));
    for i in (0..names.len()).filter(|_| !has_model) {
        planes = planes.segment_state(i, State::Disabled);
    }
    row()
        .key("toolbar")
        .role("toolbar")
        .label("Toolbar")
        .w_full()
        .gap(space::S3)
        .padding(space::S2)
        .items_center()
        .border(1.0, t.separator)
        .child(text("CAD2Maxwell").font(type_::TITLE).color(t.text))
        .child(
            group("Model").child(
                button(k, "open-step", "Open STEP…", idle)
                    .shortcut_hint("Ctrl+O")
                    .on_press_tx(open_step_dialog),
            ),
        )
        .child(
            group("Project")
                .child(
                    button(k, "save-project", "Save", has_model)
                        .shortcut_hint("Ctrl+S")
                        .on_press_tx(|s: &mut AppState, tx: &mut Tx| {
                            save_project_dialog(s, tx, false)
                        }),
                )
                .child(
                    button(k, "open-project", "Open project…", idle)
                        .on_press_tx(open_project_dialog),
                ),
        )
        .child(group("Section plane").child(planes.build(k).key("ribbon-plane")))
        .child(
            group("Run").child(
                button(k, "run-section", "Compute section", idle && has_model)
                    .shortcut_hint("Ctrl+Enter")
                    .on_press(compute),
            ),
        )
        .child(
            group("Export").child(
                button(k, "export-dxf", "Export DXF…", idle && exportable(s))
                    .shortcut_hint("Ctrl+E")
                    .on_press_tx(export_dialog),
            ),
        )
        .child(
            group("History")
                .child(
                    button(k, "undo-plane", "Undo", !s.section.past.is_empty())
                        .shortcut_hint("Ctrl+Z")
                        .on_press(|s: &mut AppState| s.section.undo()),
                )
                .child(
                    button(k, "redo-plane", "Redo", !s.section.future.is_empty())
                        .shortcut_hint("Ctrl+Y")
                        .on_press(|s: &mut AppState| s.section.redo()),
                ),
        )
        .child(spacer())
        .child(choice(
            k,
            "toggle-view",
            "View",
            &["2D", "3D"],
            Some(usize::from(s.preview.mode == ViewportMode::ThreeD)),
            |s, i| {
                s.preview.mode = [ViewportMode::TwoD, ViewportMode::ThreeD][i];
            },
        ))
}

/// Applies `change` to the number format and remembers it.
fn edit_format(change: impl Fn(&mut NumberFormat) + 'static) -> impl Fn(&mut AppState) + 'static {
    move |s| {
        let mut format = s.shell.number_format;
        change(&mut format);
        s.shell.set_format(format);
    }
}

fn stepper(
    t: &Theme,
    k: &Kit,
    key: &str,
    label: &str,
    value: u8,
    set: fn(&mut NumberFormat, u8),
) -> El {
    row()
        .w_full()
        .gap(space::S1)
        .items_center()
        .key(format!("{key}-stepper"))
        .role("group")
        .label(label)
        .child(
            text(label)
                .font(type_::LABEL)
                .color(t.text_secondary)
                .w(96.0),
        )
        .child(
            button(k, &format!("{key}-dec"), "−", value > 0)
                .label(format!("Fewer {label}"))
                .on_press(edit_format(move |f| set(f, value.saturating_sub(1)))),
        )
        .child(
            text(value.to_string())
                .font(type_::BODY)
                .color(t.text)
                .w(20.0),
        )
        .child(
            button(k, &format!("{key}-inc"), "+", value < MAX_DECIMALS)
                .label(format!("More {label}"))
                .on_press(edit_format(move |f| set(f, (value + 1).min(MAX_DECIMALS)))),
        )
}

/// Display unit, notation and theme (replaces the settings column while open).
pub fn preferences(s: &AppState, t: &Theme, k: &Kit) -> El {
    let f = s.shell.number_format;
    let themes = choice(
        k,
        "theme",
        "Theme",
        &ThemeChoice::ALL.map(|c| format!("{c:?}")),
        ThemeChoice::ALL.iter().position(|c| *c == s.shell.theme),
        |s, i| s.shell.set_theme(ThemeChoice::ALL[i]),
    );
    let units = choice(
        k,
        "unit",
        "Display unit",
        &LengthUnit::ALL.map(LengthUnit::symbol),
        LengthUnit::ALL.iter().position(|u| *u == f.length_unit),
        |s, i| edit_format(move |f| f.length_unit = LengthUnit::ALL[i])(s),
    );
    let markers = choice(
        k,
        "marker",
        "Decimal marker",
        &["Point (12.5)", "Comma (12,5)"],
        Some(usize::from(f.decimal_marker == DecimalMarker::Comma)),
        |s, i| {
            let marker = [DecimalMarker::Point, DecimalMarker::Comma][i];
            edit_format(move |f| f.decimal_marker = marker)(s)
        },
    );
    let check = |key: &str, label: &str, on: bool, set: fn(&mut NumberFormat)| {
        Checkbox::new(on)
            .label(label)
            .on_toggle(edit_format(set))
            .build(k)
            .key(key.to_string())
    };
    let resolution = NumberFormat {
        trailing_zeros: true,
        ..f
    }
    .length(10f64.powi(-i32::from(f.length_decimals)) * f.length_unit.mm_per_unit());
    let section = |title: &str| {
        text(title)
            .font(type_::SECTION)
            .color(t.text_secondary)
            .padding(space::S2)
    };
    col()
        .key("preferences")
        .role("region")
        .label("Preferences")
        .w_full()
        .child(heading(t, "Preferences"))
        .child(
            text("Display notation per ISO 80000-1; geometry and DXF stay in mm.")
                .font(type_::LABEL)
                .color(t.text_secondary)
                .wrap()
                .padding(space::S2),
        )
        .child(section("Appearance"))
        .child(themes.mx(space::S2))
        .child(section("Length"))
        .child(units.mx(space::S2))
        .child(
            col()
                .w_full()
                .padding(space::S2)
                .gap(space::S1)
                .child(stepper(
                    t,
                    k,
                    "length-decimals",
                    "Decimal places",
                    f.length_decimals,
                    |f, v| f.length_decimals = v,
                ))
                .child(label_value(t, "Resolution", resolution)),
        )
        .child(section("Direction vectors"))
        .child(col().w_full().px(space::S2).child(stepper(
            t,
            k,
            "direction-decimals",
            "Direction places",
            f.direction_decimals,
            |f, v| f.direction_decimals = v,
        )))
        .child(section("Notation"))
        .child(markers.mx(space::S2))
        .child(
            col()
                .gap(space::S1)
                .padding(space::S2)
                .child(check(
                    "grouping",
                    "Group digits in threes",
                    f.digit_grouping,
                    |f| f.digit_grouping = !f.digit_grouping,
                ))
                .child(check(
                    "trailing-zeros",
                    "Keep trailing zeros",
                    f.trailing_zeros,
                    |f| f.trailing_zeros = !f.trailing_zeros,
                )),
        )
        .child(section("Preview"))
        .child(
            col()
                .w_full()
                .px(space::S2)
                .gap(space::S1)
                .child(label_value(t, "Length", f.length(-12345.6789)))
                .child(label_value(
                    t,
                    "Size",
                    f.length_product(&[120.0, 85.5, 0.35]),
                ))
                .child(label_value(
                    t,
                    "Normal n",
                    f.direction(&[
                        std::f64::consts::FRAC_1_SQRT_2,
                        0.0,
                        -std::f64::consts::FRAC_1_SQRT_2,
                    ]),
                ))
                .child(label_value(t, "Number", f.number(1_234_567.25, 2))),
        )
        .child(
            col().padding(space::S2).child(
                button(k, "restore-format", "Restore defaults", true)
                    .on_press(|s: &mut AppState| s.shell.set_format(NumberFormat::default())),
            ),
        )
        .child(rule(t))
        .child(
            text("Values are rounded for display only; computation uses the full value.")
                .font(type_::LABEL)
                .color(t.text_secondary)
                .wrap()
                .padding(space::S2),
        )
}

pub fn messages(s: &AppState, t: &Theme, k: &Kit) -> El {
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
                    .w(52.0),
            )
            .child(text(&m.text).font(type_::BODY).color(t.text).grow(1.0));
        match focus {
            Some((id, related)) => line
                .role("button")
                .label(format!("{} {}", severity_label(m.severity), m.text))
                .focusable(true)
                .on_press(move |s: &mut AppState| s.focus_component(&id, related.clone())),
            None => line,
        }
    });
    let mut list = col().w_full();
    if let Some(busy) = &s.shell.busy {
        list = list.child(
            row()
                .key("progress")
                .role("status")
                .label(format!("{busy}…"))
                .w_full()
                .gap(space::S2)
                .px(space::S2)
                .py(2.0)
                .child(
                    text("RUN")
                        .font(type_::LABEL)
                        .color(t.text_secondary)
                        .w(44.0),
                )
                .child(
                    text(format!("{busy}... {} s", s.shell.elapsed_secs()))
                        .font(type_::BODY)
                        .color(t.text),
                ),
        );
        if s.shell.cancellable() {
            list = list.child(
                button(k, "cancel-operation", "Cancel", true).on_press(AppState::cancel_operation),
            );
        }
    }
    if s.shell.messages.is_empty() && s.shell.busy.is_none() {
        list = list.child(
            text("No messages.")
                .font(type_::BODY)
                .color(t.text_secondary)
                .px(space::S2),
        );
    }
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
        .child(scroller(k, "messages-scroll", list.children(rows)))
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
    let model = s
        .model
        .imported
        .as_ref()
        .map_or("No model".to_string(), |i| {
            let unit = i.source_unit.as_deref().unwrap_or("unknown units");
            format!("{} bodies, source {unit}", i.component_count)
        });
    row()
        .key("status")
        .role("status")
        .label("Status")
        .w_full()
        .gap(space::S3)
        .px(space::S2)
        .py(space::S1)
        .child(text(backend).font(type_::LABEL).color(backend_color))
        .child(text(model).font(type_::LABEL).color(t.text_secondary))
        .child(
            text(format!(
                "Units: {}",
                s.shell.number_format.length_unit.symbol()
            ))
            .font(type_::LABEL)
            .color(t.text_secondary),
        )
        .child(spacer())
        .child(
            text(s.shell.busy.as_deref().map_or_else(String::new, |b| {
                format!("{b}... {} s", s.shell.elapsed_secs())
            }))
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
        Key::Escape => s.model.select(None),
        Key::Text(k) if selected.is_some() && k.eq_ignore_ascii_case("v") => {
            s.model.toggle_hidden(&selected.unwrap_or_default())
        }
        Key::Text(k) if selected.is_some() && k.eq_ignore_ascii_case("i") => {
            s.model.toggle_isolate(&selected.unwrap_or_default())
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
    let mut changed = s.shell.busy.is_some();
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
