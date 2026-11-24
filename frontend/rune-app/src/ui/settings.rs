//! The settings dialog (Ctrl+,): General, Units and numbers, Viewport, Export and Service. Every
//! change applies at once and is remembered; Reset restores the shown category only.

use rune::tok::{Theme, space, type_};
use rune::{El, Tx, col, edit_access, row, text};
use rune_kit::{Checkbox, Dialog, Kit, TextInput};

use super::{button, choice, label_value, rule, scroller};
use crate::format::{DecimalMarker, LengthUnit, MAX_DECIMALS, NumberFormat};
use crate::state::AppState;
use crate::state::settings::{
    Background3d, Category, ColorMode, FILL_OPACITIES, LINE_WIDTHS, LayerMode, MousePreset,
    OPTIONAL_FORMATS, Quality, RECENT_COUNTS, Settings,
};
use crate::state::shell::Theme as ThemeChoice;

/// Applies `change` to the number format and remembers it.
fn edit_format(change: impl Fn(&mut NumberFormat) + 'static) -> impl Fn(&mut AppState) + 'static {
    move |s| {
        let mut format = s.shell.number_format;
        change(&mut format);
        s.shell.set_format(format);
    }
}

fn edit(change: impl Fn(&mut Settings) + 'static) -> impl Fn(&mut AppState) + 'static {
    move |s| s.shell.edit_settings(|x| change(x))
}

fn heading(t: &Theme, title: &str) -> El {
    text(title.to_owned())
        .font(type_::SECTION)
        .color(t.text_secondary)
        .mt(space::S2)
}

fn note(t: &Theme, msg: impl Into<String>) -> El {
    text(msg)
        .font(type_::LABEL)
        .color(t.text_secondary)
        .wrap()
        .w_full()
}

fn check(k: &Kit, key: &str, label: &str, on: bool, set: impl Fn(&mut AppState) + 'static) -> El {
    Checkbox::new(on)
        .label(label)
        .on_toggle(set)
        .build(k)
        .key(key.to_string())
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
                .w(112.0),
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

fn percent(v: f32) -> String {
    format!("{:.0} %", v * 100.0)
}

fn general(s: &AppState, t: &Theme, k: &Kit) -> El {
    let g = &s.shell.settings.general;
    let themes = choice(
        k,
        "theme",
        "Theme",
        &ThemeChoice::ALL.map(|c| format!("{c:?}")),
        ThemeChoice::ALL.iter().position(|c| *c == s.shell.theme),
        |s, i| s.shell.set_theme(ThemeChoice::ALL[i]),
    );
    let counts = choice(
        k,
        "recent-count",
        "Recent projects",
        &RECENT_COUNTS.map(|c| c.to_string()),
        RECENT_COUNTS.iter().position(|c| *c == g.recent_count),
        |s, i| {
            s.shell
                .edit_settings(|x| x.general.recent_count = RECENT_COUNTS[i])
        },
    );
    col()
        .w_full()
        .gap(space::S1)
        .child(heading(t, "Appearance"))
        .child(themes)
        .child(heading(t, "Projects"))
        .child(counts)
        .child(note(
            t,
            "How many recent projects the File group and the welcome screen list.",
        ))
        .child(check(
            k,
            "confirm-unsaved",
            "Ask before discarding unsaved changes",
            g.confirm_unsaved,
            edit(|x| x.general.confirm_unsaved = !x.general.confirm_unsaved),
        ))
}

fn units(s: &AppState, t: &Theme, k: &Kit) -> El {
    let f = s.shell.number_format;
    let unit = choice(
        k,
        "unit",
        "Display unit",
        &LengthUnit::ALL.map(LengthUnit::symbol),
        LengthUnit::ALL.iter().position(|u| *u == f.length_unit),
        |s, i| edit_format(move |f| f.length_unit = LengthUnit::ALL[i])(s),
    );
    let marker = choice(
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
    let resolution = NumberFormat {
        trailing_zeros: true,
        ..f
    }
    .length(10f64.powi(-i32::from(f.length_decimals)) * f.length_unit.mm_per_unit());
    col()
        .w_full()
        .gap(space::S1)
        .child(note(
            t,
            "Display notation per ISO 80000-1; geometry and DXF stay in mm.",
        ))
        .child(heading(t, "Length"))
        .child(unit)
        .child(stepper(
            t,
            k,
            "length-decimals",
            "Decimal places",
            f.length_decimals,
            |f, v| f.length_decimals = v,
        ))
        .child(label_value(t, "Resolution", resolution))
        .child(heading(t, "Direction vectors"))
        .child(stepper(
            t,
            k,
            "direction-decimals",
            "Direction places",
            f.direction_decimals,
            |f, v| f.direction_decimals = v,
        ))
        .child(heading(t, "Notation"))
        .child(marker)
        .child(check(
            k,
            "grouping",
            "Group digits in threes",
            f.digit_grouping,
            edit_format(|f| f.digit_grouping = !f.digit_grouping),
        ))
        .child(check(
            k,
            "trailing-zeros",
            "Keep trailing zeros",
            f.trailing_zeros,
            edit_format(|f| f.trailing_zeros = !f.trailing_zeros),
        ))
        .child(heading(t, "Preview"))
        .child(label_value(t, "Length", f.length(-12345.6789)))
        .child(label_value(
            t,
            "Size",
            f.length_product(&[120.0, 85.5, 0.35]),
        ))
        .child(label_value(t, "Number", f.number(1_234_567.25, 2)))
        .child(note(
            t,
            "Values are rounded for display only; computation uses the full value.",
        ))
}

fn viewport(s: &AppState, t: &Theme, k: &Kit) -> El {
    let v = &s.shell.settings.viewport;
    let opacity = choice(
        k,
        "fill-opacity",
        "2D fill opacity",
        &FILL_OPACITIES.map(percent),
        FILL_OPACITIES
            .iter()
            .position(|o| (*o - v.fill_opacity).abs() < 1e-3),
        |s, i| {
            s.shell
                .edit_settings(|x| x.viewport.fill_opacity = FILL_OPACITIES[i])
        },
    );
    let width = choice(
        k,
        "line-width",
        "2D line width",
        &LINE_WIDTHS.map(|w| format!("{w} px")),
        LINE_WIDTHS
            .iter()
            .position(|w| (*w - v.line_width).abs() < 1e-3),
        |s, i| {
            s.shell
                .edit_settings(|x| x.viewport.line_width = LINE_WIDTHS[i])
        },
    );
    let background = choice(
        k,
        "background-3d",
        "3D background",
        &Background3d::ALL.map(|b| format!("{b:?}")),
        Background3d::ALL.iter().position(|b| *b == v.background_3d),
        |s, i| {
            s.shell
                .edit_settings(|x| x.viewport.background_3d = Background3d::ALL[i])
        },
    );
    let quality = choice(
        k,
        "preview-quality",
        "Preview quality",
        &Quality::ALL.map(|q| format!("{q:?}")),
        Quality::ALL.iter().position(|q| *q == v.quality),
        |s, i| {
            s.shell
                .edit_settings(|x| x.viewport.quality = Quality::ALL[i])
        },
    );
    let mouse = choice(
        k,
        "mouse-preset",
        "Mouse preset",
        &MousePreset::ALL.map(MousePreset::label),
        MousePreset::ALL.iter().position(|m| *m == v.mouse),
        |s, i| {
            s.shell
                .edit_settings(|x| x.viewport.mouse = MousePreset::ALL[i])
        },
    );
    col()
        .w_full()
        .gap(space::S1)
        .child(heading(t, "2D section"))
        .child(opacity)
        .child(width)
        .child(check(
            k,
            "grid-default",
            "Show the grid when a section opens",
            v.grid_default,
            |s| {
                let on = !s.shell.settings.viewport.grid_default;
                s.shell.edit_settings(|x| x.viewport.grid_default = on);
                s.section.view.grid = on;
            },
        ))
        .child(heading(t, "3D preview"))
        .child(background)
        .child(quality)
        .child(note(
            t,
            "Quality sets the render resolution; the mesh itself comes from the service.",
        ))
        .child(heading(t, "Mouse"))
        .child(mouse)
        .child(note(t, v.mouse.description()))
}

fn export(s: &AppState, t: &Theme, k: &Kit) -> El {
    let e = &s.shell.settings.export;
    let layers = choice(
        k,
        "export-layers",
        "Layers",
        &LayerMode::ALL.map(|m| format!("{m:?}")),
        LayerMode::ALL.iter().position(|m| *m == e.layer_mode),
        |s, i| {
            s.shell
                .edit_settings(|x| x.export.layer_mode = LayerMode::ALL[i])
        },
    );
    let colors = choice(
        k,
        "export-colors",
        "Colours",
        &ColorMode::ALL.map(|m| format!("{m:?}")),
        ColorMode::ALL.iter().position(|m| *m == e.color_mode),
        |s, i| {
            s.shell
                .edit_settings(|x| x.export.color_mode = ColorMode::ALL[i])
        },
    );
    let mut formats = col().w_full().gap(space::S1).child(check(
        k,
        "format-dxf",
        "DXF (always written)",
        true,
        |_| {},
    ));
    for name in OPTIONAL_FORMATS {
        let on = e.formats.iter().any(|f| f == name);
        formats = formats.child(check(
            k,
            &format!("format-{name}"),
            name,
            on,
            edit(move |x| {
                if on {
                    x.export.formats.retain(|f| f != name);
                } else {
                    x.export.formats.push(name.to_owned());
                }
            }),
        ));
    }
    let folder = TextInput::new(&s.shell.export_folder)
        .label("Default export folder")
        .placeholder("System default")
        .width(None)
        .height(22.0)
        .on_edit(edit_access(|s: &mut AppState| &mut s.shell.export_folder))
        .on_submit(|s: &mut AppState, _| s.shell.commit_export_folder())
        .build(k)
        .key("export-folder")
        .grow(1.0);
    col()
        .w_full()
        .gap(space::S1)
        .child(heading(t, "Default folder"))
        .child(
            row()
                .w_full()
                .gap(space::S1)
                .child(folder)
                .child(
                    button(k, "export-folder-browse", "Browse…", true).on_press_tx(
                        |_: &mut AppState, tx: &mut Tx| {
                            tx.open_folder(
                                rune::FileDialogOptions {
                                    title: "Default export folder".into(),
                                    ..Default::default()
                                },
                                |s: &mut AppState, r, _| {
                                    if let Ok(Some(path)) = r {
                                        s.shell.set_export_folder(Some(path));
                                    }
                                },
                            )
                        },
                    ),
                )
                .child(button(k, "export-folder-clear", "Clear", e.folder.is_some()).on_press(
                    |s: &mut AppState| s.shell.set_export_folder(None),
                )),
        )
        .child(heading(t, "Layers and colours"))
        .child(layers)
        .child(colors)
        .child(note(
            t,
            "Group layers and colours use the groups of the model tree; bodies outside any group keep their own layer.",
        ))
        .child(heading(t, "Formats"))
        .child(formats)
        .child(heading(t, "Package"))
        .child(check(
            k,
            "export-zip",
            "Write one ZIP bundle",
            e.zip,
            edit(|x| x.export.zip = !x.export.zip),
        ))
        .child(note(t, "Off writes the files into a folder next to the chosen name."))
}

fn service(s: &AppState, t: &Theme, k: &Kit) -> El {
    let dir = crate::backend::log_dir();
    let status = match &s.shell.backend {
        crate::state::shell::BackendStatus::Online {
            service,
            api_version,
        } => {
            format!("{service} (API {api_version})")
        }
        crate::state::shell::BackendStatus::Offline(reason) => format!("Offline: {reason}"),
        crate::state::shell::BackendStatus::Connecting => "Connecting".into(),
    };
    col()
        .w_full()
        .gap(space::S1)
        .child(label_value(t, "Service", status))
        .child(label_value(t, "Log folder", dir.display().to_string()))
        .child(
            row().gap(space::S1).child(
                button(k, "open-log-folder", "Open log folder", true)
                    .on_press(|s: &mut AppState| s.shell.open_log_folder()),
            ),
        )
        .child(heading(t, "Log level"))
        .child(
            button(k, "log-level", "INFO", false)
                .label("Log level (not adjustable)"),
        )
        .child(note(
            t,
            "Not adjustable: the service writes INFO and has no runtime log setting.",
        ))
        .child(heading(t, "Restart"))
        .child(button(k, "restart-service", "Restart service", false))
        .child(note(
            t,
            "Not available: the window starts the service once and keeps its session token. Restart CAD2Maxwell to restart the service.",
        ))
}

fn page(s: &AppState, t: &Theme, k: &Kit) -> El {
    let category = s.shell.settings_category;
    let body = match category {
        Category::General => general(s, t, k),
        Category::Units => units(s, t, k),
        Category::Viewport => viewport(s, t, k),
        Category::Export => export(s, t, k),
        Category::Service => service(s, t, k),
    };
    col()
        .w_full()
        .gap(space::S2)
        .padding(space::S2)
        .child(
            row().w_full().items_center().child(
                button(
                    k,
                    "reset-category",
                    &format!("Reset {}", category.label()),
                    category != Category::Service,
                )
                .on_press(move |s: &mut AppState| s.shell.reset_category(category)),
            ),
        )
        .child(rule(t))
        .child(body)
}

/// The dialog as overlay layers; empty while closed.
pub fn overlay(s: &AppState, t: &Theme, k: &Kit) -> Vec<El> {
    let mut list = col()
        .key("settings-categories")
        .w(168.0)
        .gap(2.0)
        .padding(space::S2);
    for c in Category::ALL {
        list = list.child(
            button(k, &format!("settings-{}", c.key()), c.label(), true)
                .checked(s.shell.settings_category == c)
                .on_press(move |s: &mut AppState| s.shell.settings_category = c),
        );
    }
    let body = row()
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(list)
        .child(scroller(k, "settings-scroll", page(s, t, k)));
    let mut layer = vec![];
    Dialog::new("settings", "Settings")
        .open(s.shell.show_settings)
        .width(760.0)
        .on_close(|s: &mut AppState| s.shell.show_settings = false)
        .child(body)
        .build(k, &mut layer);
    layer
}
