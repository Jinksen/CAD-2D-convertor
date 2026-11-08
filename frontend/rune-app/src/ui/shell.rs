//! Toolbar, preferences, messages pane, status bar and global keyboard shortcuts.

use rune::tok::{Theme, space, type_};
use rune::{
    Color, El, FileDialogOptions, Key, KeyEvent, ScrollSpec, Tx, col, row, spacer, text,
};

use super::{MESSAGES_HEIGHT, button, heading, label_value, rule, severity_color, severity_label, toggle};
use crate::api::dto::{SectionResponse, Severity};
use crate::format::{DecimalMarker, LengthUnit, MAX_DECIMALS, NumberFormat};
use crate::state::preview::ViewportMode;
use crate::state::section::PlaneKind;
use crate::state::shell::{BackendStatus, Theme as ThemeChoice};
use crate::state::{AppState, now_ms};

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

/// A computed, current section whose wires are all closed and classified, with no errors.
fn exportable(s: &AppState) -> bool {
    let Some(section): Option<&SectionResponse> = s.section.last.as_ref() else {
        return false;
    };
    s.section.stats.is_some_and(|st| st.errors == 0)
        && !section.components.is_empty()
        && section.components.iter().all(|c| {
            !c.wires.is_empty() && c.wires.iter().all(|w| w.closed && w.role.is_some())
        })
}

fn export_dialog(s: &mut AppState, tx: &mut Tx) {
    if !exportable(s) {
        s.shell.log(
            Severity::Warning,
            "Compute a section whose regions are all closed before exporting.",
        );
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

fn group(t: &Theme, label: &str) -> El {
    row()
        .key(format!("group-{label}"))
        .role("group")
        .label(label)
        .gap(space::S1)
        .items_center()
        .px(space::S1)
        .border(1.0, t.separator)
}

pub fn toolbar(s: &AppState, t: &Theme, accent: Color) -> El {
    let idle = s.shell.busy.is_none();
    let has_model = s.model.imported.is_some();
    let planes = [PlaneKind::XY, PlaneKind::XZ, PlaneKind::YZ].map(|kind| {
        let name = crate::project::kind_name(kind);
        let on = has_model && s.section.plane == kind;
        let mut b = toggle(t, accent, &format!("ribbon-plane-{name}"), name, on);
        b = b.disabled(!has_model);
        b.on_press(move |s: &mut AppState| s.section.set_plane(kind, now_ms()))
    });
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
        .child(
            group(t, "Model").child(
                button(t, "open-step", "Open STEP...", idle)
                    .shortcut_hint("Ctrl+O")
                    .on_press_tx(open_step_dialog),
            ),
        )
        .child(
            group(t, "Project")
                .child(
                    button(t, "save-project", "Save", has_model)
                        .shortcut_hint("Ctrl+S")
                        .on_press_tx(|s: &mut AppState, tx: &mut Tx| save_project_dialog(s, tx, false)),
                )
                .child(
                    button(t, "open-project", "Open project...", idle)
                        .on_press_tx(open_project_dialog),
                ),
        )
        .child(group(t, "Section plane").children(planes))
        .child(
            group(t, "Run").child(
                button(t, "run-section", "Compute section", idle && has_model)
                    .shortcut_hint("Ctrl+Enter")
                    .on_press(compute),
            ),
        )
        .child(
            group(t, "Export").child(
                button(t, "export-dxf", "Export DXF...", idle && exportable(s))
                    .shortcut_hint("Ctrl+E")
                    .on_press_tx(export_dialog),
            ),
        )
        .child(
            group(t, "History")
                .child(
                    button(t, "undo-plane", "Undo", !s.section.past.is_empty())
                        .shortcut_hint("Ctrl+Z")
                        .on_press(|s: &mut AppState| s.section.undo()),
                )
                .child(
                    button(t, "redo-plane", "Redo", !s.section.future.is_empty())
                        .shortcut_hint("Ctrl+Y")
                        .on_press(|s: &mut AppState| s.section.redo()),
                ),
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
}

/// Applies `change` to the number format and remembers it.
fn edit_format(change: impl Fn(&mut NumberFormat) + 'static) -> impl Fn(&mut AppState) + 'static {
    move |s| {
        let mut format = s.shell.number_format;
        change(&mut format);
        s.shell.set_format(format);
    }
}

fn stepper(t: &Theme, key: &str, label: &str, value: u8, set: fn(&mut NumberFormat, u8)) -> El {
    row()
        .w_full()
        .gap(space::S1)
        .items_center()
        .key(format!("{key}-stepper"))
        .role("group")
        .label(label)
        .child(text(label).font(type_::LABEL).color(t.text_secondary).w(96.0))
        .child(
            button(t, &format!("{key}-dec"), "-", value > 0)
                .label(format!("Fewer {label}"))
                .on_press(edit_format(move |f| set(f, value.saturating_sub(1)))),
        )
        .child(text(value.to_string()).font(type_::BODY).color(t.text).w(20.0))
        .child(
            button(t, &format!("{key}-inc"), "+", value < MAX_DECIMALS)
                .label(format!("More {label}"))
                .on_press(edit_format(move |f| set(f, (value + 1).min(MAX_DECIMALS)))),
        )
}

fn checkbox(t: &Theme, accent: Color, key: &str, label: &str, on: bool) -> El {
    toggle(t, accent, key, label, on).role("checkbox")
}

/// Display unit, notation and theme (replaces the settings column while open).
pub fn preferences(s: &AppState, t: &Theme, accent: Color) -> El {
    let f = s.shell.number_format;
    let themes = ThemeChoice::ALL.map(|choice| {
        let name = format!("{choice:?}");
        toggle(t, accent, &format!("theme-{name}"), &name, s.shell.theme == choice)
            .role("radio")
            .on_press(move |s: &mut AppState| s.shell.set_theme(choice))
    });
    let units = LengthUnit::ALL.map(|unit| {
        toggle(t, accent, &format!("unit-{}", unit.symbol()), unit.symbol(), f.length_unit == unit)
            .role("radio")
            .on_press(edit_format(move |f| f.length_unit = unit))
    });
    let markers = [(DecimalMarker::Point, "Point (12.5)"), (DecimalMarker::Comma, "Comma (12,5)")]
        .map(|(marker, label)| {
            toggle(t, accent, &format!("marker-{marker:?}"), label, f.decimal_marker == marker)
                .role("radio")
                .on_press(edit_format(move |f| f.decimal_marker = marker))
        });
    let resolution = NumberFormat { trailing_zeros: true, ..f }
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
                .padding(space::S2),
        )
        .child(section("Appearance"))
        .child(row().role("radiogroup").label("Theme").gap(space::S1).px(space::S2).children(themes))
        .child(section("Length"))
        .child(row().role("radiogroup").label("Display unit").gap(space::S1).px(space::S2).children(units))
        .child(
            col()
                .w_full()
                .padding(space::S2)
                .gap(space::S1)
                .child(stepper(t, "length-decimals", "Decimal places", f.length_decimals, |f, v| f.length_decimals = v))
                .child(label_value(t, "Resolution", resolution)),
        )
        .child(section("Direction vectors"))
        .child(
            col().w_full().px(space::S2).child(stepper(
                t,
                "direction-decimals",
                "Direction places",
                f.direction_decimals,
                |f, v| f.direction_decimals = v,
            )),
        )
        .child(section("Notation"))
        .child(row().role("radiogroup").label("Decimal marker").gap(space::S1).px(space::S2).children(markers))
        .child(
            row()
                .gap(space::S1)
                .padding(space::S2)
                .child(
                    checkbox(t, accent, "grouping", "Group digits in threes", f.digit_grouping)
                        .on_press(edit_format(|f| f.digit_grouping = !f.digit_grouping)),
                )
                .child(
                    checkbox(t, accent, "trailing-zeros", "Keep trailing zeros", f.trailing_zeros)
                        .on_press(edit_format(|f| f.trailing_zeros = !f.trailing_zeros)),
                ),
        )
        .child(section("Preview"))
        .child(
            col()
                .w_full()
                .px(space::S2)
                .gap(space::S1)
                .child(label_value(t, "Length", f.length(-12345.6789)))
                .child(label_value(t, "Size", f.length_product(&[120.0, 85.5, 0.35])))
                .child(label_value(
                    t,
                    "Normal n",
                    f.direction(&[std::f64::consts::FRAC_1_SQRT_2, 0.0, -std::f64::consts::FRAC_1_SQRT_2]),
                ))
                .child(label_value(t, "Number", f.number(1_234_567.25, 2))),
        )
        .child(
            col().padding(space::S2).child(
                button(t, "restore-format", "Restore defaults", true)
                    .on_press(|s: &mut AppState| s.shell.set_format(NumberFormat::default())),
            ),
        )
        .child(rule(t))
        .child(
            text("Values are rounded for display only; computation uses the full value.")
                .font(type_::LABEL)
                .color(t.text_secondary)
                .padding(space::S2),
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
                .label(format!("{busy}..."))
                .w_full()
                .gap(space::S2)
                .px(space::S2)
                .py(2.0)
                .child(text("RUN").font(type_::LABEL).color(t.text_secondary).w(44.0))
                .child(text(format!("{busy}...")).font(type_::BODY).color(t.text)),
        );
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
        .child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(list.children(rows)),
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
    let model = s.model.imported.as_ref().map_or("No model".to_string(), |i| {
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
            text(format!("Units: {}", s.shell.number_format.length_unit.symbol()))
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
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
    match &ev.key {
        Key::Text(k) if has_model && matches!(k.as_str(), "1" | "2" | "3") => {
            let kind = [PlaneKind::XY, PlaneKind::XZ, PlaneKind::YZ][usize::from(k.as_bytes()[0] - b'1')];
            s.section.set_plane(kind, now_ms());
        }
        Key::Escape => s.model.select(None),
        Key::Text(k) if selected.is_some() && k.eq_ignore_ascii_case("v") => {
            s.model.toggle_hidden(&selected.unwrap_or_default())
        }
        Key::Text(k) if selected.is_some() && k.eq_ignore_ascii_case("i") => {
            s.model.toggle_isolate(&selected.unwrap_or_default())
        }
        _ => return false,
    }
    true
}
