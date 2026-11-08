//! Plane fields, bases, export gating (ports of `plane.test.ts` and the plane parts of `SectionView.test.tsx`).

use super::*;
use crate::api::dto::{SectionComponent, SectionDiagnostic, SectionWire};
use crate::format::DecimalMarker;

const BOUNDS: BoundsMm = BoundsMm {
    min_xyz: [0.0, 0.0, 0.3],
    max_xyz: [10.0, 20.0, 220.147016],
};

fn dot3(a: Vec3, b: Vec3) -> f64 {
    dot(a, b)
}

#[test]
fn a_tilted_normal_gets_an_orthonormal_frame() {
    let [n, x, y] = plane_basis(PlaneKind::Custom, [1.0, 1.0, 1.0]).expect("frame");
    for v in [n, x, y] {
        assert!((dot3(v, v) - 1.0).abs() < 1e-12);
    }
    assert!(dot3(n, x).abs() < 1e-12 && dot3(n, y).abs() < 1e-12 && dot3(x, y).abs() < 1e-12);
}

#[test]
fn axis_aligned_frames_match_the_backend() {
    assert_eq!(plane_basis(PlaneKind::XZ, [0.0; 3]).unwrap()[0], [0.0, -1.0, 0.0]);
    assert_eq!(plane_basis(PlaneKind::YZ, [0.0; 3]).unwrap()[1], [0.0, 1.0, 0.0]);
    let [n, x, y] = plane_basis(PlaneKind::Custom, [0.0, 0.0, 1.0]).unwrap();
    assert_eq!((n, x, y), ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]));
    let [_, x, _] = plane_basis(PlaneKind::Custom, [1.0, 0.0, 0.0]).unwrap();
    assert_eq!(x, [0.0, 1.0, 0.0]);
}

#[test]
fn a_zero_normal_is_rejected_instead_of_guessed() {
    assert!(plane_basis(PlaneKind::Custom, [0.0; 3]).is_none());
    let s = SectionState {
        plane: PlaneKind::Custom,
        custom_normal: [0.0; 3],
        ..Default::default()
    };
    assert!(s.request(&BOUNDS).is_none());
}

#[test]
fn custom_requests_send_the_origin_on_the_unit_normal() {
    let s = SectionState {
        plane: PlaneKind::Custom,
        custom_normal: [0.0, 3.0, 4.0],
        offset_mm: Some(10.0),
        ..Default::default()
    };
    let Some(SectionPlane::Custom {
        origin_xyz,
        normal_xyz,
        ..
    }) = s.request(&BOUNDS)
    else {
        panic!("custom request");
    };
    assert_eq!(normal_xyz, [0.0, 0.6, 0.8]);
    assert!((origin_xyz[1] - 6.0).abs() < 1e-12 && (origin_xyz[2] - 8.0).abs() < 1e-12);
}

#[test]
fn the_default_midpoint_is_rounded_to_a_micrometre_but_typed_offsets_stay_exact() {
    let mut s = SectionState::default();
    assert_eq!(s.effective_offset(&BOUNDS), 110.224);
    s.offset_text.replace_text("110.2235081");
    s.commit_fields(0);
    assert_eq!(s.offset_mm, Some(110.2235081));
    // Committing again with the same (unchanged) text adds no history and rounds nothing.
    let steps = s.past.len();
    s.commit_fields(10_000);
    assert_eq!((s.past.len(), s.offset_mm), (steps, Some(110.2235081)));
}

#[test]
fn a_decimal_comma_is_accepted_and_invalid_text_stays_out_of_the_plane() {
    let mut s = SectionState::default();
    s.fmt.set(NumberFormat {
        decimal_marker: DecimalMarker::Comma,
        ..NumberFormat::default()
    });
    s.set_offset(15.0, 0);
    assert_eq!(s.offset_text.text, "15,000");
    s.offset_text.replace_text("12,5");
    s.commit_fields(10_000);
    assert_eq!(s.offset_mm, Some(12.5));
    s.offset_text.replace_text("12,5 mm");
    s.commit_fields(20_000);
    assert_eq!(s.offset_mm, Some(12.5));
    assert!(s.field_error.as_deref().is_some_and(|e| e.contains("not a number")));
}

#[test]
fn the_offset_field_shows_the_display_unit_but_the_plane_stores_millimetres() {
    let mut s = SectionState::default();
    s.fmt.set(NumberFormat {
        length_unit: crate::format::LengthUnit::Cm,
        ..NumberFormat::default()
    });
    s.set_offset(15.0, 0);
    assert_eq!(s.offset_text.text, "1.500");
    s.offset_text.replace_text("2");
    s.commit_fields(10_000);
    assert_eq!(s.offset_mm, Some(20.0));
}

#[test]
fn normal_fields_edit_the_custom_normal_and_undo_restores_the_text() {
    let mut s = SectionState::default();
    s.set_plane(PlaneKind::Custom, 0);
    s.normal_text[0].replace_text("1");
    s.normal_text[2].replace_text("0");
    s.commit_fields(10_000);
    assert_eq!(s.custom_normal, [1.0, 0.0, 0.0]);
    s.undo();
    assert_eq!(s.custom_normal, [0.0, 0.0, 1.0]);
    assert_eq!(s.normal_text[2].text, "1.0000");
}

#[test]
fn a_custom_plane_from_the_service_is_one_undo_step() {
    let mut s = SectionState::default();
    s.set_custom([0.0, 1.0, 0.0], Some(7.0), 0);
    assert_eq!((s.plane, s.custom_normal, s.offset_mm), (PlaneKind::Custom, [0.0, 1.0, 0.0], Some(7.0)));
    s.undo();
    assert_eq!(s.plane, PlaneKind::XY);
}

fn complete_section() -> SectionResponse {
    SectionResponse {
        section_id: "s".into(),
        import_id: "i".into(),
        plane: SectionPlane::XY { offset_mm: 0.0 },
        components: vec![SectionComponent {
            component_id: "a".into(),
            wires: vec![SectionWire {
                closed: true,
                role: Some(WireRole::Outer),
                curves: vec![],
            }],
        }],
        diagnostics: vec![],
    }
}

#[test]
fn export_is_blocked_until_the_current_section_is_complete_and_says_why() {
    let mut s = SectionState::default();
    assert!(s.export_blocker().is_some_and(|m| m.contains("Compute")));
    s.last = Some(complete_section());
    assert!(s.export_blocker().is_some_and(|m| m.contains("plane changed")));
    s.stats = Some(SectionStats::default());
    assert_eq!(s.export_blocker(), None);
    s.last.as_mut().unwrap().components[0].wires[0].closed = false;
    assert!(s.export_blocker().is_some_and(|m| m.contains("gap")));
    s.last.as_mut().unwrap().components[0].wires[0].closed = true;
    s.last.as_mut().unwrap().diagnostics.push(SectionDiagnostic {
        code: "x".into(),
        severity: Severity::Error,
        message: "m".into(),
        component_id: None,
        related_component_id: None,
    });
    assert!(s.export_blocker().is_some_and(|m| m.contains("errors")));
    s.last.as_mut().unwrap().diagnostics.clear();
    s.last.as_mut().unwrap().components.clear();
    assert!(s.export_blocker().is_some_and(|m| m.contains("no bodies")));
}

#[test]
fn editing_the_plane_after_a_section_marks_it_stale() {
    let mut s = SectionState {
        last: Some(complete_section()),
        stats: Some(SectionStats::default()),
        ..Default::default()
    };
    s.set_offset(3.0, 0);
    assert!(s.stats.is_none() && s.last.is_some());
}
