//! Ribbon: captioned groups, tooltips with shortcuts, plane and Compute in one place.

use super::shell_tests::{click, motor, runtime_with};
use crate::state::section::PlaneKind;

#[test]
fn the_ribbon_has_the_five_captioned_groups_in_order() {
    let rt = runtime_with(motor());
    let x = |key: &str| rt.find(key).unwrap_or_else(|| panic!("no {key}")).0;
    let keys = [
        "group-File",
        "group-Section",
        "group-Export",
        "group-Edit",
        "group-View",
    ];
    for pair in keys.windows(2) {
        assert!(x(pair[0]) < x(pair[1]), "{} before {}", pair[0], pair[1]);
    }
    for (key, name) in [
        ("open-step", "Open a STEP file"),
        ("run-section", "Compute the exact section"),
        ("export-dxf", "Export the section as a DXF bundle"),
        ("undo-plane", "Undo the last plane change"),
    ] {
        let help = rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .and_then(|e| e.help.clone());
        assert_eq!(
            help.as_deref().map(|h| h.starts_with(name)),
            Some(true),
            "{key}: {help:?}"
        );
    }
}

#[test]
fn plane_and_compute_exist_once_and_the_ribbon_offers_custom() {
    let mut rt = runtime_with(motor());
    assert!(rt.find("section-compute").is_none() && rt.find("plane").is_none());
    assert!(
        rt.find("section-offset").is_some(),
        "offset stays in the Section pane"
    );
    click(&mut rt, "ribbon-plane/segment3");
    assert_eq!(rt.state.section.plane, PlaneKind::Custom);
    assert!(rt.find("normal-x").is_some());
}
