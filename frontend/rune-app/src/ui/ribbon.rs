//! The ribbon: File, Section, Export, Edit and View groups, each with a small caption. Every
//! button has a tooltip with its shortcut. Plane and Compute live here and nowhere else.

use rune::tok::{Theme, space, type_};
use rune::{El, Tx, col, row, spacer, text};
use rune_kit::{Kit, SegmentedControl, State};

use super::shell::{
    compute, export_dialog, exportable, open_project_dialog, open_step_dialog, save_project_dialog,
};
use super::{button, choice};
use crate::state::preview::ViewportMode;
use crate::state::section::PlaneKind;
use crate::state::{AppState, now_ms};

const PLANES: [PlaneKind; 4] = [
    PlaneKind::XY,
    PlaneKind::XZ,
    PlaneKind::YZ,
    PlaneKind::Custom,
];

/// A captioned ribbon group: its controls above, the caption below.
fn group(t: &Theme, label: &str, controls: Vec<El>) -> El {
    col()
        .key(format!("group-{label}"))
        .role("group")
        .label(label)
        .gap(2.0)
        .items_center()
        .child(row().gap(space::S1).items_center().children(controls))
        .child(text(label).font(type_::LABEL).color(t.text_secondary))
}

fn tip(el: El, what: &str, keys: &[&str]) -> El {
    el.help(what).help_keys(keys.iter().copied())
}

pub fn toolbar(s: &AppState, t: &Theme, k: &Kit) -> El {
    let idle = s.shell.busy.is_none();
    let has_model = s.model.imported.is_some();
    let names = PLANES.map(|p| match p {
        PlaneKind::Custom => "Custom",
        p => crate::project::kind_name(p),
    });
    let on = PLANES.iter().position(|p| *p == s.section.plane);
    let mut planes = SegmentedControl::new(&names, on.filter(|_| has_model))
        .label("Section plane")
        .fit()
        .on_pick(|s: &mut AppState, i, _| s.section.set_plane(PLANES[i], now_ms()));
    for i in (0..names.len()).filter(|_| !has_model) {
        planes = planes.segment_state(i, State::Disabled);
    }
    let computing = s.shell.busy.as_deref() == Some("Computing exact section");
    let file = vec![
        tip(
            button(k, "open-step", "Open STEP…", idle).on_press_tx(open_step_dialog),
            "Open a STEP file",
            &["Ctrl", "O"],
        ),
        tip(
            button(k, "open-project", "Open project…", idle).on_press_tx(open_project_dialog),
            "Open a CAD2Maxwell project",
            &[],
        ),
        tip(
            button(k, "save-project", "Save", has_model)
                .on_press_tx(|s: &mut AppState, tx: &mut Tx| save_project_dialog(s, tx, false)),
            "Save the project (Shift adds Save as)",
            &["Ctrl", "S"],
        ),
    ];
    let section = vec![
        tip(
            planes.build(k).key("ribbon-plane"),
            "Section plane. Keys 1, 2, 3 pick XY, XZ, YZ",
            &[],
        ),
        tip(
            button(
                k,
                "run-section",
                if computing {
                    "Computing…"
                } else {
                    "Compute section"
                },
                idle && has_model,
            )
            .on_press(compute),
            "Compute the exact section",
            &["Ctrl", "Enter"],
        ),
    ];
    let export = vec![tip(
        button(k, "export-dxf", "Export DXF…", idle && exportable(s)).on_press_tx(export_dialog),
        "Export the section as a DXF bundle",
        &["Ctrl", "E"],
    )];
    let edit = vec![
        tip(
            button(k, "undo-plane", "Undo", !s.section.past.is_empty())
                .on_press(|s: &mut AppState| s.section.undo()),
            "Undo the last plane change",
            &["Ctrl", "Z"],
        ),
        tip(
            button(k, "redo-plane", "Redo", !s.section.future.is_empty())
                .on_press(|s: &mut AppState| s.section.redo()),
            "Redo the plane change",
            &["Ctrl", "Y"],
        ),
    ];
    let view = vec![tip(
        choice(
            k,
            "toggle-view",
            "View",
            &["2D", "3D"],
            Some(usize::from(s.preview.mode == ViewportMode::ThreeD)),
            |s, i| {
                s.preview.mode = [ViewportMode::TwoD, ViewportMode::ThreeD][i];
            },
        ),
        "Switch between the 2D section and the 3D preview. Ctrl+1 and Ctrl+2",
        &[],
    )];
    row()
        .key("toolbar")
        .role("toolbar")
        .label("Toolbar")
        .w_full()
        .gap(space::S3)
        .padding(space::S2)
        .items_start()
        .border(1.0, t.separator)
        .child(
            text("CAD2Maxwell")
                .font(type_::TITLE)
                .color(t.text)
                .mt(space::S1),
        )
        .child(group(t, "File", file))
        .child(group(t, "Section", section))
        .child(group(t, "Export", export))
        .child(group(t, "Edit", edit))
        .child(group(t, "View", view))
        .child(spacer())
        .child(tip(
            button(k, "open-settings", "Settings", true)
                .checked(s.shell.show_prefs)
                .on_press(|s: &mut AppState| s.shell.show_prefs = !s.shell.show_prefs),
            "Display units, notation and theme",
            &[],
        ))
}
