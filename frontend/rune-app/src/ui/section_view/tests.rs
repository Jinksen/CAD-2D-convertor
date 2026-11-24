use std::time::Instant;

use rune::{Env, Mods, Runtime};

use super::*;
use crate::api::dto::{SectionCurve, SectionPlane, SectionWire};
use crate::state::section::measure::Tool;
use crate::state::section::{PlaneKind, SectionStats};

fn line(a: P, b: P) -> SectionCurve {
    SectionCurve::Line { start: a, end: b }
}

fn square(x: f64, y: f64, w: f64) -> SectionWire {
    SectionWire {
        closed: true,
        role: Some(WireRole::Outer),
        curves: vec![
            line([x, y], [x + w, y]),
            line([x + w, y], [x + w, y + w]),
            line([x + w, y + w], [x, y + w]),
            line([x, y + w], [x, y]),
        ],
    }
}

fn hole(cx: f64, cy: f64, r: f64) -> SectionWire {
    SectionWire {
        closed: true,
        role: Some(WireRole::Hole),
        curves: vec![SectionCurve::Circle {
            center: [cx, cy],
            radius: r,
            x_axis: [1.0, 0.0],
            y_axis: [0.0, 1.0],
            start_parameter: 0.0,
            end_parameter: std::f64::consts::TAU,
        }],
    }
}

fn section_of(components: Vec<SectionComponent>) -> SectionResponse {
    SectionResponse {
        section_id: "s".into(),
        import_id: "imp".into(),
        plane: SectionPlane::XY { offset_mm: 0.0 },
        components,
        diagnostics: vec![],
    }
}

/// Body "a": a 10 mm square with a 2 mm hole; body "b": a 4 mm square to its right.
fn two_bodies() -> SectionResponse {
    section_of(vec![
        SectionComponent {
            component_id: "a".into(),
            wires: vec![square(0.0, 0.0, 10.0), hole(5.0, 5.0, 2.0)],
        },
        SectionComponent {
            component_id: "b".into(),
            wires: vec![square(20.0, 0.0, 4.0)],
        },
    ])
}

/// About `curves` curves: cells of a square plus a hole circle (5 curves), 40 cells per body.
fn synthetic(curves: usize) -> SectionResponse {
    let cells = curves.div_ceil(5);
    let side = (cells as f64).sqrt().ceil() as usize;
    let mut comps: Vec<SectionComponent> = vec![];
    for i in 0..cells {
        let (x, y) = ((i % side) as f64 * 3.0, (i / side) as f64 * 3.0);
        if i % 40 == 0 {
            comps.push(SectionComponent {
                component_id: format!("c{}", i / 40),
                wires: vec![],
            });
        }
        let c = comps.last_mut().unwrap();
        c.wires.push(square(x, y, 2.0));
        c.wires.push(hole(x + 1.0, y + 1.0, 0.5));
    }
    section_of(comps)
}

/// Strokes of the drawing aids: the scale bar (3) and each origin axis in view.
fn aid_strokes(s: &AppState) -> usize {
    let size = s.section.view.size.unwrap();
    let shown = visible_box(&Map::new(view_of(s), size), size, 0.0);
    3 + usize::from(shown[0] <= 0.0 && shown[2] >= 0.0)
        + usize::from(shown[1] <= 0.0 && shown[3] >= 0.0)
}

fn state_with(section: SectionResponse) -> AppState {
    let mut s = AppState::default();
    s.section.stats = Some(SectionStats::of(&section));
    s.section.last = Some(section);
    s.section.view.size = Some(Size {
        width: 800.0,
        height: 600.0,
    });
    s
}

fn runtime(state: AppState) -> Runtime<AppState> {
    let mut rt = Runtime::new(
        state,
        |s: &AppState, cx| {
            let t = rune::tok::theme(false);
            canvas(s, &t, &crate::ui::kit(&t, cx.accent()))
        },
        (800.0, 640.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    rt.frame(0.0);
    rt.frame(1.0 / 60.0);
    rt
}

fn view_of(s: &AppState) -> ViewBox {
    s.section.camera().expect("camera")
}

fn key(s: &mut AppState, k: Key, shift: bool) -> bool {
    on_key(
        s,
        &KeyEvent {
            key: k,
            mods: Mods {
                shift,
                ..Mods::default()
            },
        },
    )
}

fn paint_of<'a>(
    s: &'a AppState,
    scene: &'a Scene,
    hidden: &'a dyn Fn(&str) -> bool,
    none: &'a dyn Fn(&str) -> Option<[u8; 3]>,
) -> Paint<'a> {
    Paint {
        section: s.section.last.as_ref().unwrap(),
        scene,
        hidden,
        color: none,
        selected: None,
        hover: None,
        related: None,
        stale: false,
        grid: s.section.view.grid,
        fill_opacity: 0.28,
        line_width: 1.0,
        fmt: crate::format::NumberFormat::default(),
        measure: &s.section.view.measure,
    }
}

#[test]
fn the_first_view_fits_the_rendered_geometry_with_margin() {
    let s = state_with(two_bodies());
    let v = view_of(&s);
    // Geometry spans x 0..24, y 0..10 (view y -10..0), margin 8% of 24.
    assert!((v.x + 1.92).abs() < 1e-9 && (v.width - (24.0 + 3.84)).abs() < 1e-9);
    assert!((v.y - (-10.0 - 1.92)).abs() < 1e-9);
}

#[test]
fn keyboard_navigation_pans_fits_and_zooms() {
    let mut s = state_with(two_bodies());
    let fitted = view_of(&s);
    key(&mut s, Key::Right, false);
    let v = view_of(&s);
    assert!((v.x - (fitted.x + fitted.width * 0.1)).abs() < 1e-9 && v.width == fitted.width);
    key(&mut s, Key::Up, false);
    assert!((view_of(&s).y - (fitted.y - fitted.height * 0.1)).abs() < 1e-9);
    key(&mut s, Key::Text("f".into()), false);
    assert_eq!(view_of(&s), fitted);
    key(&mut s, Key::Text("+".into()), false);
    assert!((view_of(&s).width - fitted.width * 0.8).abs() < 1e-9);
    key(&mut s, Key::Text("-".into()), false);
    assert!((view_of(&s).width - fitted.width).abs() < 1e-9);
    assert!(!key(&mut s, Key::Text("x".into()), false));
}

#[test]
fn fit_selection_frames_that_body_and_shift_f_frames_everything() {
    let mut s = state_with(two_bodies());
    let all = view_of(&s);
    s.model.select(Some("b".into()));
    key(&mut s, Key::Text("f".into()), false);
    assert!((view_of(&s).x - (20.0 - 0.32)).abs() < 1e-9);
    key(&mut s, Key::Text("F".into()), true);
    assert_eq!(view_of(&s), all);
}

#[test]
fn a_camera_belongs_to_its_section_and_a_new_section_is_fitted() {
    let mut s = state_with(two_bodies());
    let fitted = view_of(&s);
    s.section.zoom(0.5, None);
    assert_ne!(view_of(&s), fitted);
    s.section.last.as_mut().unwrap().section_id = "s2".into();
    assert_eq!(view_of(&s), fitted);
}

#[test]
fn wheel_zooms_about_the_cursor() {
    let mut rt = runtime(state_with(two_bodies()));
    let before = view_of(&rt.state);
    let (x, y) = rt.find("section-canvas").expect("canvas");
    let size = rt.state.section.view.size.expect("size from layout");
    // `find` gives the element center; handlers see element-local points.
    let local = [size.width / 2.0, size.height / 2.0];
    let under = |rt: &Runtime<AppState>| camera::screen_to_view(view_of(&rt.state), size, local);
    let anchor = under(&rt);
    rt.wheel_event(rune::WheelEvent::new(0.0, 40.0).at(x, y));
    rt.frame(1.0 / 60.0);
    let after = view_of(&rt.state);
    assert!((after.width - before.width * 0.9).abs() < 1e-6, "{after:?}");
    let moved = under(&rt);
    assert!((moved[0] - anchor[0]).abs() < 1e-6 && (moved[1] - anchor[1]).abs() < 1e-6);
}

#[test]
fn a_drag_pans_and_a_plain_click_selects_the_body_under_it() {
    let mut rt = runtime(state_with(two_bodies()));
    let before = view_of(&rt.state);
    let (cx, cy) = rt.find("section-canvas").expect("canvas");
    rt.pointer_down(cx, cy);
    rt.pointer_move(cx + 20.0, cy);
    rt.pointer_move(cx + 40.0, cy);
    rt.pointer_up(cx + 40.0, cy);
    rt.frame(1.0 / 60.0);
    let panned = view_of(&rt.state);
    assert!(panned.x < before.x && (panned.width - before.width).abs() < 1e-12);
    assert!(rt.state.model.selected.is_none(), "a drag is not a click");
    // Click on body b: its position in the element, offset by the element's origin.
    let size = rt.state.section.view.size.unwrap();
    let b = Map::new(panned, size).px([22.0, 2.0]);
    let origin = (cx - size.width as f32 / 2.0, cy - size.height as f32 / 2.0);
    rt.pointer_down(origin.0 + b[0], origin.1 + b[1]);
    rt.pointer_up(origin.0 + b[0], origin.1 + b[1]);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.model.selected.as_deref(), Some("b"));
}

#[test]
fn hovering_a_body_in_the_2d_view_highlights_it_and_leaving_clears_it() {
    let mut rt = runtime(state_with(two_bodies()));
    let (cx, cy) = rt.find("section-canvas").expect("canvas");
    let view = view_of(&rt.state);
    let size = rt.state.section.view.size.unwrap();
    let b = Map::new(view, size).px([22.0, 2.0]);
    let origin = (cx - size.width as f32 / 2.0, cy - size.height as f32 / 2.0);
    rt.pointer_move(origin.0 + b[0], origin.1 + b[1]);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.model.hover.as_deref(), Some("b"));
    assert!(rt.state.model.selected.is_none(), "hover never selects");
    rt.pointer_move(1.0, 1.0);
    rt.frame(1.0 / 60.0);
    assert!(rt.state.model.hover.is_none());
}

#[test]
fn picking_ignores_hidden_bodies_and_treats_a_hole_as_empty() {
    let s = state_with(two_bodies());
    let scene = s.section.scene().unwrap();
    let (view, size) = (view_of(&s), s.section.view.size.unwrap());
    let map = Map::new(view, size);
    let none = |_: &str| None;
    let shown = |_: &str| false;
    let hide_a = |id: &str| id == "a";
    let pk = |p: P, hidden: &dyn Fn(&str) -> bool| {
        let q = map.px(p);
        pick(
            &paint_of(&s, &scene, hidden, &none),
            view,
            size,
            [f64::from(q[0]), f64::from(q[1])],
        )
    };
    assert_eq!(pk([1.0, 1.0], &shown).as_deref(), Some("a"));
    assert_eq!(pk([5.0, 5.0], &shown), None, "inside the hole");
    assert_eq!(
        pk([5.0, 7.0], &shown).as_deref(),
        Some("a"),
        "on the hole edge"
    );
    assert_eq!(pk([1.0, 1.0], &hide_a), None);
    assert_eq!(pk([-5.0, -5.0], &shown), None);
}

#[test]
fn holes_are_bridged_out_of_the_fill_and_errors_suppress_fills_but_not_outlines() {
    let mut s = state_with(two_bodies());
    let look = Look::of(&rune::tok::theme(false));
    let build = |s: &AppState| {
        let scene = s.section.scene().unwrap();
        let (hidden, none) = (|_: &str| false, |_: &str| None);
        build_shape(
            &paint_of(s, &scene, &hidden, &none),
            view_of(s),
            s.section.view.size.unwrap(),
            &look,
        )
    };
    let filled = format!("{:?}", build(&s));
    s.section
        .last
        .as_mut()
        .unwrap()
        .diagnostics
        .push(crate::api::dto::SectionDiagnostic {
            code: "x".into(),
            severity: Severity::Error,
            message: "bad".into(),
            component_id: None,
            related_component_id: None,
        });
    let outlined = format!("{:?}", build(&s));
    // Fills: the square with its hole bridged out, and the second square; strokes in both cases.
    assert_eq!(filled.matches("Fill {").count(), 2);
    assert_eq!(outlined.matches("Fill {").count(), 0);
    assert_eq!(outlined.matches("Stroke {").count(), 3 + aid_strokes(&s));
}

#[test]
fn off_screen_wires_are_culled() {
    let mut s = state_with(two_bodies());
    let look = Look::of(&rune::tok::theme(false));
    let count = |s: &AppState| {
        let scene = s.section.scene().unwrap();
        let (hidden, none) = (|_: &str| false, |_: &str| None);
        let shape = build_shape(
            &paint_of(s, &scene, &hidden, &none),
            view_of(s),
            s.section.view.size.unwrap(),
            &look,
        );
        format!("{shape:?}").matches("Stroke {").count() - aid_strokes(s)
    };
    assert_eq!(count(&s), 3);
    // Zoom in on body b: body a (and its hole) are gone.
    s.model.select(Some("b".into()));
    key(&mut s, Key::Text("f".into()), false);
    assert_eq!(count(&s), 1);
}

fn report(label: &str, section: SectionResponse) {
    let mut rt = runtime(state_with(section));
    let n: usize = rt
        .state
        .section
        .last
        .as_ref()
        .unwrap()
        .components
        .iter()
        .flat_map(|c| &c.wires)
        .map(|w| w.curves.len())
        .sum();
    let (x, y) = rt.find("section-canvas").unwrap();
    let mut frame_ms = vec![];
    for i in 0..60 {
        // Alternate wheel zoom in/out about the cursor: each event rebuilds the view.
        let dy = if i % 2 == 0 { 40.0 } else { -40.0 };
        rt.wheel_event(rune::WheelEvent::new(0.0, dy).at(x, y));
        let t0 = Instant::now();
        rt.frame(1.0 / 60.0);
        frame_ms.push(t0.elapsed().as_secs_f64() * 1e3);
    }
    let look = Look::of(&rune::tok::theme(false));
    let scene = rt.state.section.scene().unwrap();
    let (hidden, none) = (|_: &str| false, |_: &str| None);
    let t0 = Instant::now();
    let shape = build_shape(
        &paint_of(&rt.state, &scene, &hidden, &none),
        view_of(&rt.state),
        rt.state.section.view.size.unwrap(),
        &look,
    );
    let build_ms = t0.elapsed().as_secs_f64() * 1e3;
    let t1 = Instant::now();
    let _ = Scene::of(rt.state.section.last.as_ref().unwrap());
    let scene_ms = t1.elapsed().as_secs_f64() * 1e3;
    let ops = format!("{shape:?}");
    frame_ms.sort_by(f64::total_cmp);
    println!(
        "BENCH {label} curves={n} shape_build_ms={build_ms:.2} scene_once_ms={scene_ms:.2} frame_median_ms={:.2} frame_p95_ms={:.2} ops={}",
        frame_ms[frame_ms.len() / 2],
        frame_ms[frame_ms.len() * 95 / 100],
        ops.matches("Stroke {").count() + ops.matches("Fill {").count()
    );
}

/// `cargo test --release bench_ -- --ignored --nocapture`
#[test]
#[ignore = "timing report, meaningful in release builds only"]
fn bench_synthetic_sections() {
    for curves in [100, 5_000, 100_000] {
        report("synthetic", synthetic(curves));
    }
}

/// Needs the running service: CAD2MAXWELL_BACKEND_URL, CAD2MAXWELL_SESSION_TOKEN, CAD2MAXWELL_TEST_STEP.
#[test]
#[ignore = "needs the running geometry service"]
fn bench_real_sections() {
    let step = std::env::var("CAD2MAXWELL_TEST_STEP").expect("CAD2MAXWELL_TEST_STEP");
    let client = crate::api::Client::from_env(std::env::var("CAD2MAXWELL_SESSION_TOKEN").ok());
    let summary = client.import_step(&step).unwrap();
    let mut state = crate::state::section::SectionState::default();
    for kind in [PlaneKind::XY, PlaneKind::XZ, PlaneKind::YZ] {
        state.plane = kind;
        state.offset_mm = None;
        let plane = state.request(&summary.bounds_mm).unwrap();
        let t0 = Instant::now();
        let section = client.section(&summary.import_id, &plane).unwrap();
        println!(
            "BENCH service section {kind:?} took {:.0} ms",
            t0.elapsed().as_secs_f64() * 1e3
        );
        report(&format!("real-{kind:?}"), section);
    }
}

fn strokes(s: &AppState) -> usize {
    let scene = s.section.scene().unwrap();
    let (hidden, none) = (|_: &str| false, |_: &str| None);
    let look = Look::of(&rune::tok::theme(false));
    let shape = build_shape(
        &paint_of(s, &scene, &hidden, &none),
        view_of(s),
        s.section.view.size.unwrap(),
        &look,
    );
    format!("{shape:?}").matches("Stroke {").count()
}

#[test]
fn the_grid_toggle_adds_lines_and_the_scale_bar_is_always_drawn() {
    let mut s = state_with(two_bodies());
    let off = strokes(&s);
    s.section.view.grid = true;
    assert!(strokes(&s) > off + 4, "grid lines under the geometry");
    let mut rt = runtime(state_with(two_bodies()));
    let (x, y) = rt.find("toggle-grid").expect("grid button");
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert!(rt.state.section.view.grid);
    assert!(rt.find("section-scale").is_some());
}

#[test]
fn the_readout_shows_cursor_coordinates_in_the_display_unit_and_the_hovered_name() {
    let mut s = state_with(two_bodies());
    s.model
        .set_imported(crate::state::model::tests::summary(&["a", "b"]));
    s.shell.number_format.length_unit = crate::format::LengthUnit::Cm;
    assert!(overlay::readout(&s).is_none(), "no pointer, no readout");
    s.section.view.cursor = Some([20.0, 5.0]);
    let text = overlay::readout(&s).unwrap();
    assert!(
        text.contains("2.000\u{a0}cm") && text.contains("0.500\u{a0}cm"),
        "{text}"
    );
    s.model.hover = Some("b".into());
    assert!(overlay::readout(&s).unwrap().ends_with("b_1"));
}

#[test]
fn pointer_moves_track_the_cursor_in_section_coordinates() {
    let mut rt = runtime(state_with(two_bodies()));
    let (cx, cy) = rt.find("section-canvas").expect("canvas");
    let size = rt.state.section.view.size.unwrap();
    let origin = (cx - size.width as f32 / 2.0, cy - size.height as f32 / 2.0);
    let at = Map::new(view_of(&rt.state), size).px([22.0, 2.0]);
    rt.pointer_move(origin.0 + at[0], origin.1 + at[1]);
    rt.frame(1.0 / 60.0);
    let c = rt.state.section.view.cursor.expect("cursor");
    assert!(
        (c[0] - 22.0).abs() < 0.1 && (c[1] - 2.0).abs() < 0.1,
        "{c:?}"
    );
    assert!(rt.find("section-readout").is_some());
}

#[test]
fn the_legend_lists_visible_groups_with_their_color() {
    let mut s = state_with(two_bodies());
    s.model
        .set_imported(crate::state::model::tests::summary(&["a", "b"]));
    s.model.groups = vec![
        crate::state::model::BodyGroup {
            name: "Stator".into(),
            component_ids: vec!["a".into()],
        },
        crate::state::model::BodyGroup {
            name: "Rotor".into(),
            component_ids: vec!["b".into()],
        },
    ];
    assert_eq!(overlay::legend(&s).len(), 2);
    assert_eq!(
        overlay::legend(&s)[0],
        ("Stator".to_string(), Some([255, 0, 0]))
    );
    s.model.toggle_group_hidden("Rotor");
    let names: Vec<_> = overlay::legend(&s).into_iter().map(|e| e.0).collect();
    assert_eq!(names, ["Stator"]);
}

fn click_at(rt: &mut Runtime<AppState>, world: P) {
    let (cx, cy) = rt.find("section-canvas").expect("canvas");
    let size = rt.state.section.view.size.unwrap();
    let origin = (cx - size.width as f32 / 2.0, cy - size.height as f32 / 2.0);
    let p = Map::new(view_of(&rt.state), size).px(world);
    rt.pointer_down(origin.0 + p[0], origin.1 + p[1]);
    rt.pointer_up(origin.0 + p[0], origin.1 + p[1]);
    rt.frame(1.0 / 60.0);
}

#[test]
fn the_distance_tool_snaps_to_corners_and_reads_the_distance_in_the_display_unit() {
    let mut s = state_with(two_bodies());
    s.shell.number_format.length_unit = crate::format::LengthUnit::Cm;
    let mut rt = runtime(s);
    assert!(rt.find("measure-reading").is_none(), "Select shows no hint");
    assert!(
        rt.find("tool-2d").is_some(),
        "tool control in the 2D toolbar"
    );
    rt.state.section.view.tool = Tool::Distance;
    rt.invalidate();
    rt.frame(1.0 / 60.0);
    assert!(rt.find("measure-reading").is_some(), "the tool hint");
    // Near (not on) the corners (0,0) and (10,10): both snap.
    click_at(&mut rt, [0.05, -0.04]);
    assert_eq!(rt.state.section.view.measure.first, Some([0.0, 0.0]));
    assert!(rt.state.model.selected.is_none(), "measuring never selects");
    click_at(&mut rt, [10.03, 9.97]);
    let text = overlay::measure_text(&rt.state).unwrap();
    assert!(text.starts_with("Distance 1.414\u{a0}cm"), "{text}");
    assert!(text.contains("dx 1.000\u{a0}cm"), "{text}");
}

#[test]
fn the_radius_tool_reads_a_circle_and_reports_a_miss() {
    let mut rt = runtime(state_with(two_bodies()));
    rt.state.section.view.tool = Tool::Radius;
    click_at(&mut rt, [7.0, 5.0]);
    let text = overlay::measure_text(&rt.state).unwrap();
    assert!(text.starts_with("Radius 2.000\u{a0}mm"), "{text}");
    assert!(text.contains("(5.000\u{a0}mm, 5.000\u{a0}mm)"), "{text}");
    click_at(&mut rt, [1.0, 1.0]);
    assert_eq!(
        overlay::measure_text(&rt.state).as_deref(),
        Some("No arc or circle under the pointer.")
    );
}

#[test]
fn measure_marks_add_strokes_over_the_geometry() {
    let mut s = state_with(two_bodies());
    let before = strokes(&s);
    s.section.view.measure.click_distance([0.0, 0.0]);
    s.section.view.measure.click_distance([10.0, 10.0]);
    assert_eq!(strokes(&s), before + 5, "two crosses and the line");
}
