//! 2D section drawing. `viewport` calls `canvas` in 2D mode.
//!
//! Curves stay analytic in state; each view build tessellates only what is on screen, at a
//! tolerance of a fraction of a pixel, so arcs look round at any zoom and the section data (and
//! the DXF export) are never touched. Rune fills are simple polygons without holes, so each hole
//! is bridged into its outer ring (`fill`) to leave it genuinely empty.

use rune::tok::{Theme, space, type_};
use rune::{
    Color, Drag, El, Key, KeyEvent, LineCap, MagnifyEvent, Phase, PointerButton, PointerEvent,
    Shape, Tx, WheelEvent, canvas as draw, col, row, text,
};

use rune_kit::Kit;

use super::{button, choice};
use crate::api::dto::{SectionComponent, SectionResponse, SectionWire, Severity, WireRole};
use crate::format::NumberFormat;
use crate::state::AppState;
use crate::state::section::Scene;
use crate::state::section::camera::{self, Size, ViewBox};
use crate::state::section::geometry::{self, outline_color};
use crate::state::section::measure::Measure;
use crate::state::section::measure::Tool;

const PAN_DRAG_PX: f32 = 3.0;
/// Pointer tolerance for picking a curve, in pixels.
const PICK_PX: f64 = 4.0;
/// A measurement point snaps to a curve end or arc centre within this many pixels.
const SNAP_PX: f64 = 8.0;
/// Wires smaller than this on screen are not drawn.
const MIN_WIRE_PX: f64 = 0.3;
type P = [f64; 2];

struct Look {
    dark: bool,
    /// Grid lines, origin axes and scale bar ink.
    faint: Color,
    axis_x: Color,
    axis_y: Color,
    ink: Color,
    selected: Color,
    related: Color,
    default: Color,
}

fn rgb(c: [u8; 3]) -> Color {
    rune::color::hex(&format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])).unwrap_or(Color::WHITE)
}

impl Look {
    fn of(t: &Theme) -> Look {
        let dark = t.bg.luminance() < 0.5;
        Look {
            dark,
            faint: t.text_secondary.alpha(0.18),
            axis_x: rgb(if dark { [230, 100, 100] } else { [190, 40, 40] }).alpha(0.7),
            axis_y: rgb(if dark { [100, 200, 120] } else { [30, 130, 60] }).alpha(0.7),
            ink: t.text_secondary,
            selected: rgb(if dark { [98, 160, 234] } else { [26, 95, 180] }),
            related: rgb(if dark { [255, 163, 72] } else { [179, 92, 0] }),
            default: rgb(if dark {
                [120, 160, 200]
            } else {
                [74, 111, 143]
            }),
        }
    }
}

/// A wire ready to draw: its screen-space polylines.
struct Drawn {
    role: Option<WireRole>,
    closed: bool,
    lines: Vec<Vec<[f32; 2]>>,
}

/// Maps section points (y up) to element pixels.
struct Map {
    view: ViewBox,
    scale: f64,
    off: P,
}

impl Map {
    fn new(view: ViewBox, size: Size) -> Map {
        let scale = camera::scale(view, size);
        Map {
            view,
            scale,
            off: [
                (size.width - view.width * scale) / 2.0,
                (size.height - view.height * scale) / 2.0,
            ],
        }
    }
    fn px(&self, p: P) -> [f32; 2] {
        [
            ((p[0] - self.view.x) * self.scale + self.off[0]) as f32,
            ((-p[1] - self.view.y) * self.scale + self.off[1]) as f32,
        ]
    }
    /// Display tolerance in section units.
    fn tol(&self) -> f64 {
        0.2 / self.scale
    }
}

fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

/// The section rectangle shown (y up), grown by `margin_px`.
fn visible_box(map: &Map, size: Size, margin_px: f64) -> [f64; 4] {
    let m = margin_px / map.scale;
    let tl = camera::screen_to_view(map.view, size, [0.0, 0.0]);
    let br = camera::screen_to_view(map.view, size, [size.width, size.height]);
    [tl[0] - m, -br[1] - m, br[0] + m, -tl[1] + m]
}

fn wire_box(wire: &SectionWire) -> Option<[f64; 4]> {
    wire.curves.iter().map(geometry::curve_box).reduce(|a, b| {
        [
            a[0].min(b[0]),
            a[1].min(b[1]),
            a[2].max(b[2]),
            a[3].max(b[3]),
        ]
    })
}

/// The wires of a body that touch the view and are larger than a speck, tessellated to pixels.
fn draw_wires(comp: &SectionComponent, map: &Map, shown: [f64; 4]) -> Vec<Drawn> {
    let mut out = vec![];
    for wire in &comp.wires {
        let Some(b) = wire_box(wire) else { continue };
        if !overlaps(b, shown) || (b[2] - b[0]).max(b[3] - b[1]) * map.scale < MIN_WIRE_PX {
            continue;
        }
        let (lines, closed) = geometry::wire_polylines(wire, map.tol());
        out.push(Drawn {
            role: wire.role,
            closed,
            lines: lines
                .iter()
                .map(|l| l.iter().map(|&p| map.px(p)).collect())
                .collect(),
        });
    }
    out
}

/// What the painter needs to know about the shown section and its bodies.
pub struct Paint<'a> {
    pub section: &'a SectionResponse,
    pub scene: &'a Scene,
    pub hidden: &'a dyn Fn(&str) -> bool,
    pub color: &'a dyn Fn(&str) -> Option<[u8; 3]>,
    pub selected: Option<&'a str>,
    pub related: Option<&'a str>,
    /// Body under the pointer in the tree or this view: display highlight only.
    pub hover: Option<&'a str>,
    pub stale: bool,
    pub grid: bool,
    pub fmt: NumberFormat,
    pub measure: &'a Measure,
}

fn build_shape(p: &Paint, view: ViewBox, size: Size, look: &Look) -> Shape {
    let map = Map::new(view, size);
    let shown = visible_box(&map, size, 4.0);
    let s = p.section;
    let fills_ok = s.diagnostics.iter().all(|d| d.severity != Severity::Error);
    let drawn: Vec<Vec<Drawn>> = s
        .components
        .iter()
        .map(|c| draw_wires(c, &map, shown))
        .collect();
    let dim = if p.stale { 0.4 } else { 1.0 };
    // (fill, outline) colors of a body.
    let base = |id: &str| -> (Color, Color) {
        if p.selected == Some(id) {
            (look.selected, look.selected)
        } else if p.related == Some(id) || p.hover == Some(id) {
            (look.related, look.related)
        } else if let Some(c) = (p.color)(id) {
            (rgb(c), rgb(outline_color(c, look.dark)))
        } else {
            (look.default, look.default)
        }
    };
    let mut shape = overlay::under(Shape::new(), &map, size, look, p.grid);
    if fills_ok {
        for &i in &p.scene.paint_order {
            let comp = &s.components[i];
            let complete = comp.wires.iter().all(|w| w.closed && w.role.is_some());
            if !complete || (p.hidden)(&comp.component_id) {
                continue;
            }
            let fill_color = base(&comp.component_id).0.alpha(0.28 * dim);
            for (pts, holes) in fill::rings(&drawn[i]) {
                let cut: Vec<&[P]> = holes.iter().map(|h| h.as_slice()).collect();
                if let Some(poly) = fill::with_holes(&pts, &cut) {
                    let poly: Vec<[f32; 2]> =
                        poly.iter().map(|p| [p[0] as f32, p[1] as f32]).collect();
                    shape = shape.fill(poly, fill_color);
                }
            }
        }
    }
    // Strokes: ordinary bodies first, then the related and selected ones on top.
    let rank = |id: &str| {
        u8::from(p.related == Some(id) || p.hover == Some(id))
            + 2 * u8::from(p.selected == Some(id))
    };
    let mut order: Vec<usize> = (0..s.components.len()).collect();
    order.sort_by_key(|&i| rank(&s.components[i].component_id));
    for i in order {
        let id = &s.components[i].component_id;
        let alpha = if (p.hidden)(id) { 0.2 } else { dim };
        let width = if rank(id) == 0 { 1.0 } else { 2.0 };
        let color = base(id).1.alpha(alpha);
        for w in &drawn[i] {
            for line in &w.lines {
                shape = shape.stroke_with(line.clone(), w.closed, width, LineCap::Round, color);
            }
        }
    }
    let shape = overlay::marks(shape, &map, look, p.measure);
    overlay::over(shape, &map, size, look, &p.fmt)
}

/// The body under a screen point: a curve within a few pixels, else the smallest region.
pub fn pick(p: &Paint, view: ViewBox, size: Size, at: P) -> Option<String> {
    let map = Map::new(view, size);
    let w = camera::screen_to_view(view, size, at);
    let world = [w[0], -w[1]];
    let reach = PICK_PX / map.scale;
    let near = [
        world[0] - reach,
        world[1] - reach,
        world[0] + reach,
        world[1] + reach,
    ];
    for &i in p.scene.paint_order.iter().rev() {
        let comp = &p.section.components[i];
        if (p.hidden)(&comp.component_id) {
            continue;
        }
        let mut parity = false;
        for wire in &comp.wires {
            if !wire_box(wire).is_some_and(|b| overlaps(b, near)) {
                continue;
            }
            let (lines, closed) = geometry::wire_polylines(wire, map.tol());
            let on_curve = lines.iter().any(|l| {
                l.windows(2)
                    .any(|s| geometry::segment_distance(s[0], s[1], world) <= reach)
            });
            if on_curve {
                return Some(comp.component_id.clone());
            }
            if closed && geometry::inside(&lines[0], world) {
                parity = !parity;
            }
        }
        if parity {
            return Some(comp.component_id.clone());
        }
    }
    None
}

fn paint<'a>(
    s: &'a AppState,
    section: &'a SectionResponse,
    scene: &'a Scene,
    hidden: &'a dyn Fn(&str) -> bool,
    color: &'a dyn Fn(&str) -> Option<[u8; 3]>,
) -> Paint<'a> {
    Paint {
        section,
        scene,
        hidden,
        color,
        selected: s.model.selected.as_deref(),
        related: s.model.related.as_deref(),
        hover: s.model.hover.as_deref(),
        stale: s.section.stats.is_none(),
        grid: s.section.view.grid,
        fmt: s.shell.number_format,
        measure: &s.section.view.measure,
    }
}

fn on_key(s: &mut AppState, e: &KeyEvent) -> bool {
    match &e.key {
        Key::Left => s.section.pan_fraction(-0.1, 0.0),
        Key::Right => s.section.pan_fraction(0.1, 0.0),
        Key::Up => s.section.pan_fraction(0.0, -0.1),
        Key::Down => s.section.pan_fraction(0.0, 0.1),
        Key::Text(k) => match k.as_str() {
            "f" | "F" => {
                let id = if e.mods.shift {
                    None
                } else {
                    s.model.selected.clone()
                };
                s.section.fit(id.as_deref());
            }
            "+" | "=" => s.section.zoom(0.8, None),
            "-" | "_" => s.section.zoom(1.25, None),
            _ => return false,
        },
        _ => return false,
    }
    true
}

fn on_drag(s: &mut AppState, d: &mut Drag, tx: &mut Tx) {
    let middle = tx.button() == PointerButton::Middle;
    let moved = d.dx.hypot(d.dy) > PAN_DRAG_PX;
    match d.phase {
        Phase::Down => s.section.view.drag_origin = s.section.camera(),
        Phase::Move => {
            if let (Some(origin), Some(size), true) = (
                s.section.view.drag_origin,
                s.section.view.size,
                moved || middle,
            ) {
                let k = camera::units_per_pixel(origin, size);
                let (dx, dy) = (f64::from(d.dx), f64::from(d.dy));
                s.section
                    .set_camera(camera::pan_view(origin, -dx * k, -dy * k));
            }
        }
        Phase::Up => {
            s.section.view.drag_origin = None;
            if !moved && !middle && !d.cancelled() {
                click(s, [f64::from(d.x), f64::from(d.y)]);
            }
        }
    }
}

/// The body under the pointer in the shown section.
fn body_at(s: &AppState, at: P) -> Option<String> {
    let (section, scene, view, size) = (
        s.section.last.as_ref()?,
        s.section.scene()?,
        s.section.camera()?,
        s.section.view.size?,
    );
    let hidden = |id: &str| s.model.hidden.contains(id);
    let none = |_: &str| None;
    pick(&paint(s, section, &scene, &hidden, &none), view, size, at)
}

/// Highlights the body in the tree too and scrolls its row into view.
pub(super) fn hover_from_view(s: &mut AppState, id: Option<String>, tx: &mut Tx) {
    if s.model.hover_body(id.clone()) {
        tx.scroll_into_view(format!("body-{}", id.unwrap_or_default()));
    }
}

fn click(s: &mut AppState, at: P) {
    if measuring::click(s, at) {
        return;
    }
    let (Some(section), Some(scene), Some(view), Some(size)) = (
        &s.section.last,
        s.section.scene(),
        s.section.camera(),
        s.section.view.size,
    ) else {
        return;
    };
    let hidden = |id: &str| s.model.hidden.contains(id);
    let none = |_: &str| None;
    let id = pick(&paint(s, section, &scene, &hidden, &none), view, size, at);
    if let Some(id) = id {
        s.model.select(Some(id));
    }
}

pub fn canvas(s: &AppState, t: &Theme, k: &Kit) -> El {
    let note = |msg: &str| {
        text(msg.to_owned())
            .font(type_::LABEL)
            .color(t.text_secondary)
    };
    let Some(section) = &s.section.last else {
        return col()
            .key("section-canvas")
            .w_full()
            .grow(1.0)
            .center()
            .child(note(if s.model.imported.is_some() {
                "No section computed. Choose a plane and press Compute section."
            } else {
                "Open or drop a STEP file."
            }));
    };
    let scene = s.section.scene();
    let (view, size) = (s.section.camera(), s.section.view.size);
    let look = Look::of(t);
    let imported = s.model.imported.as_ref();
    let hidden = |id: &str| s.model.hidden.contains(id);
    let color = |id: &str| {
        imported
            .and_then(|i| i.components.iter().find(|c| c.id == id))
            .and_then(|c| c.source_color)
    };
    let shape = match (&scene, view, size) {
        (Some(scene), Some(view), Some(size)) => build_shape(
            &paint(s, section, scene, &hidden, &color),
            view,
            size,
            &look,
        ),
        _ => Shape::new(),
    };
    let errors = section
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    let status = if s.section.stats.is_none() {
        "Stale: the plane changed. Compute the section to refresh."
    } else if errors {
        "The section has errors: regions are not filled. See Messages."
    } else if scene.as_ref().is_none_or(|sc| sc.bounds.is_none()) {
        "The plane does not intersect any body."
    } else {
        "Wheel: zoom, drag: pan, arrows: pan, F: fit selection, Shift+F: fit all, click: select"
    };
    let tools = row()
        .w_full()
        .gap(space::S1)
        .px(space::S2)
        .py(space::S1)
        .items_center()
        .child(
            button(k, "fit-view", "Fit", true).on_press(|s: &mut AppState| {
                let id = s.model.selected.clone();
                s.section.fit(id.as_deref());
            }),
        )
        .child(
            button(k, "zoom-in", "+", true).on_press(|s: &mut AppState| s.section.zoom(0.8, None)),
        )
        .child(
            button(k, "zoom-out", "−", true)
                .on_press(|s: &mut AppState| s.section.zoom(1.25, None)),
        )
        .child(choice(
            k,
            "tool-2d",
            "2D tool",
            &Tool::ALL.map(Tool::label),
            Tool::ALL.iter().position(|t| *t == s.section.view.tool),
            |s, i| {
                s.section.view.tool = Tool::ALL[i];
                s.section.view.measure.clear();
            },
        ))
        .child(
            button(k, "toggle-grid", "Grid", true)
                .checked(s.section.view.grid)
                .on_press(|s: &mut AppState| s.section.view.grid = !s.section.view.grid),
        )
        .child(note(status));
    let surface = draw(shape)
        .key("section-canvas")
        .label("2D section geometry")
        .focusable(true)
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .clip_soft(0.0)
        .on_layout(|s: &mut AppState, r: [f32; 4]| {
            let size = Size {
                width: f64::from(r[2]),
                height: f64::from(r[3]),
            };
            if s.section.view.size != Some(size) {
                s.section.view.size = Some(size);
            }
        })
        .on_wheel(|s: &mut AppState, e: WheelEvent, _| {
            let factor = if e.precise {
                (1.0 - f64::from(e.dy) * 0.005).clamp(0.5, 2.0)
            } else if e.dy > 0.0 {
                0.9
            } else {
                1.1
            };
            s.section
                .zoom(factor, Some([f64::from(e.x), f64::from(e.y)]));
            true
        })
        .on_magnify(|s: &mut AppState, e: MagnifyEvent, _| {
            let factor = 1.0 / (1.0 + f64::from(e.delta)).max(0.1);
            s.section
                .zoom(factor, Some([f64::from(e.x), f64::from(e.y)]));
            true
        })
        .on_key(|s: &mut AppState, e: &KeyEvent, _| on_key(s, e))
        .on_pointer_move_local(|s: &mut AppState, e: PointerEvent, tx: &mut Tx| {
            let at = [f64::from(e.x), f64::from(e.y)];
            s.section.view.cursor = s
                .section
                .camera()
                .zip(s.section.view.size)
                .map(|(v, size)| camera::screen_to_view(v, size, at))
                .map(|w| [w[0], -w[1]]);
            let id = body_at(s, at);
            hover_from_view(s, id, tx);
        })
        .on_hover_change(|s: &mut AppState, over, _| {
            if !over {
                s.section.view.cursor = None;
                s.model.hover = None;
            }
        })
        .pointer_buttons([PointerButton::Middle])
        .on_drag(on_drag);
    let map = view.zip(size).map(|(v, sz)| Map::new(v, sz));
    let area = col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(surface)
        .children(overlay::labels(s, t, map.as_ref(), size));
    col().w_full().grow(1.0).min_h(0.0).child(tools).child(area)
}

mod fill;
mod measuring;
mod overlay;
#[cfg(test)]
mod tests;
