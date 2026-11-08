//! Section plane settings, their undo/redo history, and the last computed section.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rune::TextEdit;

use crate::api::dto::{BoundsMm, SectionPlane, SectionResponse, Severity, Vec3, WireRole};
use crate::format::{NumberFormat, parse_number};

pub mod camera;
pub mod geometry;

use camera::{Size, ViewBox};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaneKind {
    #[default]
    XY,
    XZ,
    YZ,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneSnapshot {
    pub kind: PlaneKind,
    pub offset_mm: Option<f64>,
    pub custom_normal: Vec3,
}

/// Edits closer together than this form one undo step, so typing a value is one step.
pub const HISTORY_COALESCE_MS: u64 = 600;
const HISTORY_LIMIT: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SectionStats {
    pub regions: usize,
    pub errors: usize,
    pub warnings: usize,
}

impl SectionStats {
    pub fn of(section: &SectionResponse) -> Self {
        let count = |s: Severity| {
            section
                .diagnostics
                .iter()
                .filter(|d| d.severity == s)
                .count()
        };
        SectionStats {
            regions: section
                .components
                .iter()
                .flat_map(|c| &c.wires)
                .filter(|w| w.closed && w.role == Some(WireRole::Outer))
                .count(),
            errors: count(Severity::Error),
            warnings: count(Severity::Warning),
        }
    }
}

/// What the 2D view derives once per computed section (keyed by `section_id`).
pub struct Scene {
    pub section_id: String,
    /// Bounds of all curves (y up).
    pub bounds: Option<ViewBox>,
    /// Per-component bounds, parallel to `components`.
    pub component_bounds: Vec<Option<ViewBox>>,
    /// Component indices, largest bounds first, so holes painted over a body never hide a smaller
    /// body nested inside.
    pub paint_order: Vec<usize>,
}

impl Scene {
    pub fn of(section: &SectionResponse) -> Scene {
        let component_bounds: Vec<Option<ViewBox>> = section
            .components
            .iter()
            .map(|c| geometry::bounds(c.wires.iter().flat_map(|w| &w.curves)))
            .collect();
        let area = |i: usize| component_bounds[i].map_or(0.0, |b| b.width * b.height);
        let mut paint_order: Vec<usize> = (0..component_bounds.len()).collect();
        paint_order.sort_by(|&a, &b| area(b).total_cmp(&area(a)));
        let all = geometry::bounds(
            section
                .components
                .iter()
                .flat_map(|c| &c.wires)
                .flat_map(|w| &w.curves),
        );
        Scene {
            section_id: section.section_id.clone(),
            bounds: all,
            component_bounds,
            paint_order,
        }
    }
}

/// The 2D view: element size, the camera for the current section, and the derived scene.
#[derive(Default)]
pub struct View2d {
    pub size: Option<Size>,
    /// The user's camera and the section it belongs to; a new section starts from a fit.
    camera: Option<(String, ViewBox)>,
    /// Camera at the start of a drag.
    pub drag_origin: Option<ViewBox>,
    scene: RefCell<Option<Rc<Scene>>>,
}

pub struct SectionState {
    pub plane: PlaneKind,
    pub offset_mm: Option<f64>,
    pub custom_normal: Vec3,
    pub past: Vec<PlaneSnapshot>,
    pub future: Vec<PlaneSnapshot>,
    /// Time (ms, caller's monotonic clock) of the last plane edit, for coalescing.
    last_edit_ms: Option<u64>,
    pub last: Option<SectionResponse>,
    pub stats: Option<SectionStats>,
    pub view: View2d,
    /// Text of the plane fields: offset, then the custom normal components.
    pub offset_text: TextEdit,
    pub normal_text: [TextEdit; 3],
    /// Why the last field entry was not applied.
    pub field_error: Option<String>,
    /// Notation of the fields, mirrored from the shell by the settings panel.
    pub fmt: Cell<NumberFormat>,
}

impl Default for SectionState {
    fn default() -> Self {
        let mut s = SectionState {
            plane: PlaneKind::XY,
            offset_mm: None,
            custom_normal: [0.0, 0.0, 1.0],
            past: vec![],
            future: vec![],
            last_edit_ms: None,
            last: None,
            stats: None,
            view: View2d::default(),
            offset_text: TextEdit::default(),
            normal_text: Default::default(),
            field_error: None,
            fmt: Cell::new(NumberFormat::default()),
        };
        s.write_fields();
        s
    }
}

/// Unit vector. The length is computed like JS `Math.hypot` (V8: scaled, Kahan-summed) so the
/// custom plane sent to the service is bit-identical to the React app's.
fn norm(v: Vec3) -> Option<Vec3> {
    let max = v.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    let length = if max == 0.0 || !max.is_finite() {
        max
    } else {
        let (mut sum, mut compensation) = (0.0_f64, 0.0_f64);
        for x in v {
            let n = x.abs() / max;
            let summand = n * n - compensation;
            let next = sum + summand;
            compensation = (next - sum) - summand;
            sum = next;
        }
        sum.sqrt() * max
    };
    (length.is_finite() && length >= 1e-12).then(|| v.map(|x| x / length))
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// (normal, x, y) of a plane. Principal frames match the backend, so 2D coordinates agree with the DXF.
pub fn plane_basis(kind: PlaneKind, normal: Vec3) -> Option<[Vec3; 3]> {
    match kind {
        PlaneKind::XY => Some([[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
        PlaneKind::XZ => Some([[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]),
        PlaneKind::YZ => Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        PlaneKind::Custom => custom_frame(normal).map(|(n, x)| [n, x, cross(n, x)]),
    }
}

/// A deterministic in-plane x direction: global X (or Y for planes facing X).
fn custom_frame(normal: Vec3) -> Option<(Vec3, Vec3)> {
    let n = norm(normal)?;
    let reference: Vec3 = if n[0].abs() > 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let along = dot(reference, n);
    let x = norm(std::array::from_fn(|i| reference[i] - along * n[i]))?;
    Some((n, x))
}

impl SectionState {
    fn snapshot(&self) -> PlaneSnapshot {
        PlaneSnapshot {
            kind: self.plane,
            offset_mm: self.offset_mm,
            custom_normal: self.custom_normal,
        }
    }

    fn apply(&mut self, s: PlaneSnapshot) {
        self.plane = s.kind;
        self.offset_mm = s.offset_mm;
        self.custom_normal = s.custom_normal;
        self.stats = None;
        self.write_fields();
    }

    /// Call before a plane edit: pushes the current plane unless this edit continues the previous one.
    fn record(&mut self, now_ms: u64) {
        let continuing = self
            .last_edit_ms
            .is_some_and(|t| now_ms.saturating_sub(t) < HISTORY_COALESCE_MS);
        self.last_edit_ms = Some(now_ms);
        if continuing {
            return;
        }
        self.past.push(self.snapshot());
        if self.past.len() > HISTORY_LIMIT {
            self.past.remove(0);
        }
        self.future.clear();
    }

    pub fn set_plane(&mut self, kind: PlaneKind, now_ms: u64) {
        if self.plane == kind {
            return;
        }
        self.record(now_ms);
        self.plane = kind;
        self.offset_mm = None;
        self.stats = None;
        self.write_fields();
    }

    pub fn set_offset(&mut self, offset_mm: f64, now_ms: u64) {
        self.record(now_ms);
        self.offset_mm = Some(offset_mm);
        self.stats = None;
        self.write_fields();
    }

    pub fn set_custom(&mut self, normal: Vec3, offset_mm: Option<f64>, now_ms: u64) {
        self.record(now_ms);
        self.plane = PlaneKind::Custom;
        self.custom_normal = normal;
        self.offset_mm = offset_mm;
        self.stats = None;
        self.write_fields();
    }

    /// Rewrites the field texts from the plane values (after undo, restore or a button edit).
    fn write_fields(&mut self) {
        let f = self.fmt.get();
        let unit = f.length_unit.mm_per_unit();
        self.offset_text.replace_text(
            self.offset_mm
                .map_or(String::new(), |v| f.number(v / unit, f.length_decimals)),
        );
        for (edit, v) in self.normal_text.iter_mut().zip(self.custom_normal) {
            edit.replace_text(f.number(v, f.direction_decimals));
        }
        self.field_error = None;
    }

    /// Applies the typed field texts to the plane. A field still showing the stored value (to
    /// display precision) is left alone, so exact values are never rounded by merely committing.
    pub fn commit_fields(&mut self, now_ms: u64) {
        let f = self.fmt.get();
        let unit = f.length_unit.mm_per_unit();
        let mut error = None;
        let mut offset = self.offset_mm;
        let text = self.offset_text.text.trim();
        if !text.is_empty() {
            match parse_number(text) {
                Some(v) => {
                    let mm = v * unit;
                    let tol = 0.5 * 10f64.powi(-i32::from(f.length_decimals)) * unit;
                    if offset.is_none_or(|old| (old - mm).abs() > tol) {
                        offset = Some(mm);
                    }
                }
                None => error = Some(format!("Offset \"{text}\" is not a number.")),
            }
        }
        let mut normal = self.custom_normal;
        let tol = 0.5 * 10f64.powi(-i32::from(f.direction_decimals));
        for (i, edit) in self.normal_text.iter().enumerate() {
            match parse_number(&edit.text) {
                Some(v) if (v - normal[i]).abs() > tol => normal[i] = v,
                Some(_) => {}
                None => {
                    error = Some(format!(
                        "Normal component \"{}\" is not a number.",
                        edit.text.trim()
                    ));
                }
            }
        }
        if offset != self.offset_mm || normal != self.custom_normal {
            self.record(now_ms);
            self.offset_mm = offset;
            self.custom_normal = normal;
            self.stats = None;
        }
        self.field_error = error;
    }

    /// Exact basis (normal, x, y) of the current plane.
    pub fn basis(&self) -> Option<[Vec3; 3]> {
        plane_basis(self.plane, self.custom_normal)
    }

    /// Why the shown section cannot be exported, if it cannot.
    pub fn export_blocker(&self) -> Option<&'static str> {
        let Some(section) = &self.last else {
            return Some("Compute a section before exporting.");
        };
        if self.stats.is_none() {
            return Some(
                "The plane changed since the last computation. Compute the section again before exporting.",
            );
        }
        if section.components.is_empty() {
            return Some("The section contains no bodies. Move the plane through the model.");
        }
        if section
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
        {
            return Some(
                "The section has errors. Resolve the errors listed in Messages before exporting.",
            );
        }
        let closed = section
            .components
            .iter()
            .all(|c| !c.wires.is_empty() && c.wires.iter().all(|w| w.closed && w.role.is_some()));
        (!closed).then_some(
            "A body has an open or unclassified wire. Check its gap diagnostics in Messages; gaps are never closed silently.",
        )
    }

    /// The scene of the shown section, derived once per `section_id`.
    pub fn scene(&self) -> Option<Rc<Scene>> {
        let section = self.last.as_ref()?;
        let mut slot = self.view.scene.borrow_mut();
        if slot
            .as_ref()
            .is_none_or(|s| s.section_id != section.section_id)
        {
            *slot = Some(Rc::new(Scene::of(section)));
        }
        slot.clone()
    }

    /// The view box in use: the user's camera for this section, else a fit of everything.
    pub fn camera(&self) -> Option<ViewBox> {
        let id = &self.last.as_ref()?.section_id;
        match &self.view.camera {
            Some((owner, vb)) if owner == id => Some(*vb),
            _ => self
                .scene()?
                .bounds
                .map(|b| camera::fit_view(camera::from_world(b))),
        }
    }

    pub fn set_camera(&mut self, vb: ViewBox) {
        if let Some(section) = &self.last {
            self.view.camera = Some((section.section_id.clone(), vb));
        }
    }

    /// Fits one body (when it is in this section) or everything.
    pub fn fit(&mut self, component_id: Option<&str>) {
        let (Some(section), Some(scene)) = (&self.last, self.scene()) else {
            return;
        };
        let own = component_id
            .and_then(|id| section.components.iter().position(|c| c.component_id == id))
            .and_then(|i| scene.component_bounds[i]);
        if let Some(b) = own.or(scene.bounds) {
            self.set_camera(camera::fit_view(camera::from_world(b)));
        }
    }

    /// Zooms by `factor` (below 1 zooms in) about a screen point, or the center when `None`.
    pub fn zoom(&mut self, factor: f64, at: Option<[f64; 2]>) {
        let (Some(view), Some(size)) = (self.camera(), self.view.size) else {
            return;
        };
        let anchor = at.map(|p| camera::screen_to_view(view, size, p));
        self.set_camera(camera::zoom_view(view, factor, anchor));
    }

    /// Pans by a fraction of the view (arrow keys).
    pub fn pan_fraction(&mut self, fx: f64, fy: f64) {
        if let Some(v) = self.camera() {
            self.set_camera(camera::pan_view(v, fx * v.width, fy * v.height));
        }
    }

    pub fn undo(&mut self) {
        self.travel(true);
    }

    pub fn redo(&mut self) {
        self.travel(false);
    }

    fn travel(&mut self, back: bool) {
        let current = self.snapshot();
        let (from, to) = if back {
            (&mut self.past, &mut self.future)
        } else {
            (&mut self.future, &mut self.past)
        };
        let Some(target) = from.pop() else { return };
        to.push(current);
        self.last_edit_ms = None;
        self.apply(target);
    }

    /// Replaces the plane without a history step (project open or new import); history is cleared.
    pub fn restore(&mut self, plane: PlaneSnapshot) {
        self.last_edit_ms = None;
        self.past.clear();
        self.future.clear();
        self.apply(plane);
        self.last = None;
    }

    /// The offset in use: the explicit one, else the model midpoint along the plane axis.
    pub fn effective_offset(&self, bounds: &BoundsMm) -> f64 {
        // The midpoint is an arbitrary start, so it is rounded to 1 um; typed offsets stay exact.
        self.offset_mm.unwrap_or_else(|| {
            let c = bounds.center();
            let mid = match self.plane {
                PlaneKind::XY => c[2],
                PlaneKind::XZ => c[1],
                PlaneKind::YZ => c[0],
                PlaneKind::Custom => norm(self.custom_normal).map_or(0.0, |n| dot(n, c)),
            };
            (mid * 1000.0).round() / 1000.0
        })
    }

    /// The request for the current plane; `None` when a custom normal is degenerate.
    pub fn request(&self, bounds: &BoundsMm) -> Option<SectionPlane> {
        let offset_mm = self.effective_offset(bounds);
        Some(match self.plane {
            PlaneKind::XY => SectionPlane::XY { offset_mm },
            PlaneKind::XZ => SectionPlane::XZ { offset_mm },
            PlaneKind::YZ => SectionPlane::YZ { offset_mm },
            PlaneKind::Custom => {
                let (n, x) = custom_frame(self.custom_normal)?;
                SectionPlane::Custom {
                    origin_xyz: n.map(|v| v * offset_mm),
                    normal_xyz: n,
                    x_dir_xyz: x,
                }
            }
        })
    }

    pub fn plane_matches(&self, plane: &SectionPlane) -> bool {
        matches!(
            (self.plane, plane),
            (PlaneKind::XY, SectionPlane::XY { .. })
                | (PlaneKind::XZ, SectionPlane::XZ { .. })
                | (PlaneKind::YZ, SectionPlane::YZ { .. })
                | (PlaneKind::Custom, SectionPlane::Custom { .. })
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: BoundsMm = BoundsMm {
        min_xyz: [0.0, 0.0, 0.0],
        max_xyz: [10.0, 20.0, 30.0],
    };

    #[test]
    fn clicking_the_active_plane_keeps_its_offset_and_another_plane_resets_it() {
        let mut s = SectionState {
            offset_mm: Some(12.5),
            ..Default::default()
        };
        s.set_plane(PlaneKind::XY, 0);
        assert_eq!(s.offset_mm, Some(12.5));
        assert!(s.past.is_empty());
        s.set_plane(PlaneKind::XZ, 0);
        assert_eq!(s.offset_mm, None);
    }

    #[test]
    fn edits_within_the_coalescing_window_are_one_undo_step() {
        let mut s = SectionState::default();
        s.set_offset(1.0, 1000);
        s.set_offset(2.0, 1100);
        s.set_offset(3.0, 1200);
        assert_eq!(s.past.len(), 1);
        s.set_offset(9.0, 1200 + HISTORY_COALESCE_MS);
        assert_eq!(s.past.len(), 2);
        s.undo();
        assert_eq!(s.offset_mm, Some(3.0));
        s.undo();
        assert_eq!(s.offset_mm, None);
        s.redo();
        assert_eq!(s.offset_mm, Some(3.0));
        s.set_offset(5.0, 100_000);
        assert!(s.future.is_empty(), "a new edit clears redo");
    }

    #[test]
    fn undo_after_travel_never_coalesces_and_history_is_bounded() {
        let mut s = SectionState::default();
        for i in 0..150u32 {
            s.set_offset(f64::from(i), u64::from(i) * 10_000);
        }
        assert_eq!(s.past.len(), HISTORY_LIMIT);
        s.undo();
        s.set_offset(1.0, 1_500_000);
        assert_eq!(s.future.len(), 0);
    }

    #[test]
    fn restore_clears_history_and_stats() {
        let mut s = SectionState::default();
        s.set_offset(4.0, 0);
        s.stats = Some(SectionStats::default());
        s.restore(PlaneSnapshot {
            kind: PlaneKind::YZ,
            offset_mm: Some(2.0),
            custom_normal: [0.0, 0.0, 1.0],
        });
        assert!(s.past.is_empty() && s.future.is_empty() && s.stats.is_none());
        assert_eq!((s.plane, s.offset_mm), (PlaneKind::YZ, Some(2.0)));
    }

    #[test]
    fn default_offset_is_the_midpoint_and_custom_frames_are_deterministic() {
        let mut s = SectionState::default();
        assert_eq!(
            s.request(&BOUNDS),
            Some(SectionPlane::XY { offset_mm: 15.0 })
        );
        s.plane = PlaneKind::YZ;
        assert_eq!(
            s.request(&BOUNDS),
            Some(SectionPlane::YZ { offset_mm: 5.0 })
        );
        s.plane = PlaneKind::Custom;
        s.custom_normal = [0.0, 0.0, 2.0];
        assert_eq!(
            s.request(&BOUNDS),
            Some(SectionPlane::Custom {
                origin_xyz: [0.0, 0.0, 15.0],
                normal_xyz: [0.0, 0.0, 1.0],
                x_dir_xyz: [1.0, 0.0, 0.0]
            })
        );
        s.custom_normal = [0.0; 3];
        assert_eq!(s.request(&BOUNDS), None);
    }
}

#[cfg(test)]
mod plane_tests;
