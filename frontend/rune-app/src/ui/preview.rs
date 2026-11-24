//! Viewport: the 2D section drawing, or the 3D display mesh rendered on the CPU (`render3d`)
//! into a `Pixels` image. Frames are drawn on demand only: `draw` skips when its key is unchanged.

use rune::tok::{Theme, space, type_};
use rune::{
    Drag, El, Key, KeyEvent, Phase, PointerButton, PointerEvent, SurfaceFit, Tx, WheelEvent, col,
    el, row, text,
};

use rune_kit::Kit;

use super::preview_aids as aids;
use super::{button, heading};
use crate::api::dto::BoundsMm;
use crate::render3d::raster::{Body, Line, Scene, draw_triad, guide_outline};
use crate::render3d::{StandardView, plane_guide, triad};
use crate::state::AppState;
use crate::state::preview::{MeshStatus, RenderKey, ViewportMode};

/// Default body color when the source has none (Three.js scene's 0x78a9ca).
const DEFAULT_COLOR: [u8; 3] = [0x78, 0xa9, 0xca];
/// Selected body tint (0x1a3f6b emissive in the React scene).
const SELECTED_TINT: [f32; 3] = [26.0, 63.0, 107.0];
const CLICK_SLOP: f32 = 4.0;

pub fn viewport(s: &AppState, t: &Theme, k: &Kit) -> El {
    let three_d = s.preview.mode == ViewportMode::ThreeD;
    let body = if three_d {
        canvas(s, t)
    } else {
        super::section_view::canvas(s, t, k)
    };
    let mut c = col()
        .key("viewport")
        .role("region")
        .label("Viewport")
        .grow(1.0)
        .h_full()
        .min_w(0.0)
        .child(heading(
            t,
            if three_d {
                "Preview (3D)"
            } else {
                "Section (2D)"
            },
        ));
    if three_d {
        c = c.child(toolbar(s, k));
    }
    c.child(body)
}

fn toolbar(s: &AppState, k: &Kit) -> El {
    let ready = s.preview.status == MeshStatus::Ready;
    let mut r = row()
        .key("view-toolbar")
        .role("toolbar")
        .label("3D view")
        .w_full()
        .gap(space::S1)
        .px(space::S2)
        .py(space::S1);
    r = r.child(
        button(k, "view-fit", "Fit", ready).on_press(|s: &mut AppState| {
            if let Some(bounds) = bounds(s) {
                let selected = s.model.selected.clone();
                s.preview.fit(selected.as_deref(), &bounds);
            }
        }),
    );
    for view in StandardView::ALL {
        r = r.child(
            button(k, &format!("view-{}", view.label()), view.label(), ready).on_press(
                move |s: &mut AppState| {
                    if let Some(bounds) = bounds(s) {
                        s.preview.view(view, &bounds);
                    }
                },
            ),
        );
    }
    r.child(el().grow(1.0))
        .child(button(k, "view-help", "?", true).help(aids::HINT))
}

fn bounds(s: &AppState) -> Option<BoundsMm> {
    Some(s.model.imported.as_ref()?.bounds_mm.clone())
}

fn canvas(s: &AppState, t: &Theme) -> El {
    let base = col()
        .key("preview3d")
        .role("img")
        .label("3D model preview")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .on_drawn_layout(|s: &mut AppState, r| {
            s.preview.viewport = (r[2], r[3]);
            s.ensure_preview();
        });
    let message = match &s.preview.status {
        _ if s.model.imported.is_none() => "Open or drop a STEP file.".to_string(),
        MeshStatus::Empty | MeshStatus::Loading => "Generating 3D preview…".to_string(),
        MeshStatus::Failed(message) => message.clone(),
        MeshStatus::Ready => {
            draw(s, t);
            return base
                .pixels_fit(&s.preview.pixels, SurfaceFit::Fill)
                .children(aids::labels(s, t))
                .on_wheel(|s: &mut AppState, e: WheelEvent, _| {
                    s.preview.camera.zoom(f64::from(e.dy));
                    true
                })
                .focusable(true)
                .on_key(|s: &mut AppState, e: &KeyEvent, _| on_key(s, e))
                .on_pointer_move_local(|s: &mut AppState, e: PointerEvent, tx: &mut Tx| {
                    let (w, h) = s.preview.viewport;
                    let id = (w > 0.0 && h > 0.0)
                        .then(|| body_at(s, e.x / w, e.y / h))
                        .flatten();
                    super::section_view::hover_from_view(s, id, tx);
                })
                .on_hover_change(|s: &mut AppState, over, _| {
                    if !over {
                        s.model.hover = None;
                    }
                })
                .pointer_buttons([PointerButton::Middle, PointerButton::Secondary])
                .on_drag(drag);
        }
    };
    base.center().child(
        text(message)
            .font(type_::BODY)
            .color(t.text_secondary)
            .padding(space::S3),
    )
}

/// F fits the selected body, Shift+F the whole model (as in the 2D view).
fn on_key(s: &mut AppState, e: &KeyEvent) -> bool {
    let Key::Text(k) = &e.key else { return false };
    if !k.eq_ignore_ascii_case("f") {
        return false;
    }
    if let Some(bounds) = bounds(s) {
        let id = if e.mods.shift {
            None
        } else {
            s.model.selected.clone()
        };
        s.preview.fit(id.as_deref(), &bounds);
    }
    true
}

/// Left drag orbits (a short one is a click that picks), other buttons pan.
fn drag(s: &mut AppState, d: &mut Drag, tx: &mut Tx) {
    let pan = tx.button() != PointerButton::Primary;
    let p = &mut s.preview;
    match d.phase {
        Phase::Down => p.drag_from = p.camera,
        _ if d.cancelled() => p.camera = p.drag_from,
        _ => {
            let (dx, dy, h) = (f64::from(d.dx), f64::from(d.dy), f64::from(d.h));
            let mut camera = p.drag_from;
            if pan {
                camera.pan(dx, dy, h);
            } else {
                camera.orbit(dx, dy, h);
            }
            p.camera = camera;
            if d.phase == Phase::Up && !pan && d.dx.hypot(d.dy) < CLICK_SLOP {
                pick(s, d.x / d.w, d.y / d.h);
            }
        }
    }
}

/// The body under the point given as a fraction of the viewport.
fn body_at(s: &AppState, fx: f32, fy: f32) -> Option<String> {
    let frame = s.preview.frame.borrow();
    let (x, y) = (
        (fx * frame.width as f32) as usize,
        (fy * frame.height as f32) as usize,
    );
    let body = frame.pick(x, y)?;
    let mesh = s.preview.mesh.as_ref()?;
    mesh.components
        .get(body as usize)
        .map(|c| c.component_id.clone())
}

/// Selects the body under the point given as a fraction of the viewport.
fn pick(s: &mut AppState, fx: f32, fy: f32) {
    if let Some(id) = body_at(s, fx, fy) {
        s.model.select(Some(id));
    }
}

/// Renders the current state into `preview.pixels` unless the last frame is still current.
fn draw(s: &AppState, t: &Theme) {
    let p = &s.preview;
    let (Some(mesh), Some(imported)) = (&p.mesh, &s.model.imported) else {
        return;
    };
    let scale = if p.scale.get() > 0.0 {
        p.scale.get()
    } else {
        1.0
    };
    let size = (
        (p.viewport.0 * scale).round() as usize,
        (p.viewport.1 * scale).round() as usize,
    );
    if size.0 == 0 || size.1 == 0 {
        return;
    }
    let bounds = &imported.bounds_mm;
    let guide = s
        .section
        .request(bounds)
        .map(|plane| plane_guide(bounds, &plane));
    let key = RenderKey {
        size,
        camera: p.camera,
        hidden: s.model.hidden.clone(),
        selected: s.model.selected.clone(),
        hover: s.model.hover.clone(),
        guide,
        background: [t.bg.r, t.bg.g, t.bg.b].map(|c| (c * 255.0).round() as u8),
    };
    if p.drawn.borrow().as_ref() == Some(&key) {
        return;
    }
    let started = std::time::Instant::now();
    let bodies = mesh
        .components
        .iter()
        .enumerate()
        .filter(|(_, c)| !key.hidden.contains(&c.component_id))
        .map(|(i, c)| Body {
            id: i as u32,
            positions: &c.positions,
            normals: &c.normals,
            indices: &c.indices,
            edges: &c.edge_indices,
            color: c.color.unwrap_or(DEFAULT_COLOR).map(f32::from),
            emissive: if key.selected.as_deref() == Some(c.component_id.as_str()) {
                SELECTED_TINT
            } else if key.hover.as_deref() == Some(c.component_id.as_str()) {
                SELECTED_TINT.map(|v| v * 0.5)
            } else {
                [0.0; 3]
            },
        })
        .collect();
    let dark = key.background.iter().map(|&c| u32::from(c)).sum::<u32>() < 384;
    let mut lines = helpers(bounds, dark);
    if let Some(g) = guide {
        lines.extend(guide_outline(g.corners()));
    }
    let scene = Scene {
        bodies,
        lines,
        guide: guide.map(|g| g.corners()),
        background: key.background,
        edge_color: [0x1f, 0x23, 0x28],
    };
    let mut rgba = p
        .frame
        .borrow_mut()
        .render(size.0, size.1, &p.camera, &scene);
    let inset = aids::TRIAD_INSET * scale;
    draw_triad(
        &mut rgba,
        size.0,
        size.1,
        [inset, size.1 as f32 - inset],
        triad(&p.camera, f64::from(aids::TRIAD_LEN as f32 * scale)),
    );
    p.pixels.set(size.0 as u32, size.1 as u32, rgba);
    p.last_frame_ms
        .set(started.elapsed().as_secs_f32() * 1000.0);
    *p.drawn.borrow_mut() = Some(key);
}

/// The React scene's ground grid (10 divisions over twice the model diagonal, at the model's
/// lowest Z) and axes at the model center.
fn helpers(bounds: &BoundsMm, dark: bool) -> Vec<Line> {
    let diagonal = (0..3)
        .map(|i| (bounds.max_xyz[i] - bounds.min_xyz[i]).powi(2))
        .sum::<f64>()
        .sqrt();
    let radius = diagonal.max(1.0);
    let (major, minor) = if dark {
        ([80, 86, 94], [55, 60, 66])
    } else {
        ([0xb9, 0xbe, 0xc4], [0xdf, 0xe2, 0xe5])
    };
    let z = bounds.min_xyz[2];
    let mut lines = vec![];
    for k in -5..=5 {
        let at = f64::from(k) * radius / 5.0;
        let color = if k == 0 { major } else { minor };
        lines.push(Line {
            a: [at, -radius, z],
            b: [at, radius, z],
            color,
            on_top: false,
        });
        lines.push(Line {
            a: [-radius, at, z],
            b: [radius, at, z],
            color,
            on_top: false,
        });
    }
    let c = bounds.center();
    for (axis, color) in [[255, 0, 0], [0, 255, 0], [0, 0, 255]]
        .into_iter()
        .enumerate()
    {
        let mut b = c;
        b[axis] += radius * 0.2;
        lines.push(Line {
            a: c,
            b,
            color,
            on_top: false,
        });
    }
    lines
}

#[cfg(test)]
mod tests {
    use rune::{Env, Runtime};

    use super::*;
    use crate::api::dto::{PreviewComponentMesh, PreviewMesh};
    use crate::render3d::raster::tests::cube;
    use crate::state::model::tests::summary;

    fn runtime() -> Runtime<AppState> {
        let mut state = AppState::default();
        state.model.set_imported(summary(&["a"]));
        let (positions, normals, indices, edge_indices) = cube(10.0, [0.0, 5.0, 10.0]);
        state.preview.mesh = Some(
            PreviewMesh {
                import_id: "imp".into(),
                components: vec![PreviewComponentMesh {
                    component_id: "a".into(),
                    color: Some([200, 50, 50]),
                    positions,
                    normals,
                    indices,
                    edge_indices,
                }],
                deflection_mm: 0.1,
                coarsened: false,
            }
            .into(),
        );
        state.preview.status = MeshStatus::Ready;
        state.preview.mode = ViewportMode::ThreeD;
        let bounds = state.model.imported.as_ref().unwrap().bounds_mm.clone();
        state.preview.view(StandardView::Iso, &bounds);
        let mut rt = Runtime::new(state, crate::ui::view, (1280.0, 760.0), Env::new(true, 1.0));
        for _ in 0..4 {
            rt.frame(1.0 / 60.0);
        }
        rt
    }

    #[test]
    fn the_viewport_renders_on_demand_and_a_click_selects_the_body_under_the_pointer() {
        let mut rt = runtime();
        let (w, h) = rt.state.preview.pixels.size();
        assert!(w > 400 && h > 300, "rendered at the viewport size: {w}x{h}");
        let (x, y) = rt.find("preview3d").expect("viewport");
        assert!(rt.state.model.selected.is_none());
        rt.pointer_move(x, y);
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        assert_eq!(rt.state.model.selected.as_deref(), Some("a"));

        // An unchanged state draws nothing new; a camera change draws once.
        let drawn = rt.state.preview.drawn.borrow().clone();
        rt.frame(1.0 / 60.0);
        assert_eq!(*rt.state.preview.drawn.borrow(), drawn);
        let before = rt.state.preview.camera;
        rt.pointer_move(x, y);
        rt.pointer_down(x, y);
        rt.pointer_move(x + 60.0, y + 30.0);
        rt.pointer_up(x + 60.0, y + 30.0);
        rt.frame(1.0 / 60.0);
        assert_ne!(rt.state.preview.camera.yaw, before.yaw, "drag orbits");
        assert_ne!(*rt.state.preview.drawn.borrow(), drawn);
    }

    #[test]
    fn f_fits_the_selection_and_shift_f_fits_everything() {
        let mut rt = runtime();
        let s = &mut rt.state;
        let key = |s: &mut AppState, k: &str, shift: bool| {
            let mods = rune::Mods {
                shift,
                ..rune::Mods::default()
            };
            on_key(
                s,
                &KeyEvent {
                    key: Key::Text(k.into()),
                    mods,
                },
            )
        };
        s.model.select(Some("a".into()));
        s.preview.camera.pan(50.0, 50.0, 100.0);
        let panned = s.preview.camera;
        assert!(key(s, "f", false));
        assert_ne!(s.preview.camera, panned);
        s.preview.camera = panned;
        assert!(key(s, "F", true));
        assert_ne!(s.preview.camera, panned);
        assert!(!key(s, "x", false));
    }

    #[test]
    fn the_plane_label_triad_letters_and_hint_tooltip_are_present() {
        let mut rt = runtime();
        rt.state.shell.number_format.length_unit = crate::format::LengthUnit::Cm;
        rt.state.section.offset_mm = Some(25.0);
        rt.invalidate();
        rt.frame(1.0 / 60.0);
        for key in ["plane-label", "triad-x", "triad-y", "triad-z", "view-help"] {
            assert!(rt.find(key).is_some(), "{key}");
        }
        let label = super::aids::plane_label(&rt.state).unwrap();
        assert_eq!(label, "XY plane, z = 2.500\u{a0}cm");
        assert!(super::aids::mesh_note(&rt.state).is_none());
        rt.state.preview.mesh = rt.state.preview.mesh.take().map(|m| {
            std::sync::Arc::new(PreviewMesh {
                coarsened: true,
                ..(*m).clone()
            })
        });
        assert!(
            super::aids::mesh_note(&rt.state)
                .unwrap()
                .contains("exact geometry")
        );
    }

    #[test]
    fn hidden_bodies_are_not_picked() {
        let mut rt = runtime();
        rt.state.model.toggle_hidden("a");
        rt.invalidate();
        rt.frame(1.0 / 60.0);
        let (x, y) = rt.find("preview3d").expect("viewport");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        assert!(rt.state.model.selected.is_none());
    }
}
