//! Section plane settings, their undo/redo history, and the last computed section.

use crate::api::dto::{BoundsMm, SectionPlane, SectionResponse, Severity, Vec3, WireRole};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaneKind {
    #[default]
    XY,
    XZ,
    YZ,
    #[allow(dead_code)] // custom-normal editing arrives with the section settings feature
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
}

impl Default for SectionState {
    fn default() -> Self {
        SectionState {
            plane: PlaneKind::XY,
            offset_mm: None,
            custom_normal: [0.0, 0.0, 1.0],
            past: vec![],
            future: vec![],
            last_edit_ms: None,
            last: None,
            stats: None,
        }
    }
}

fn norm(v: Vec3) -> Option<Vec3> {
    let length = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    (length.is_finite() && length >= 1e-12).then(|| v.map(|x| x / length))
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
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
    }

    pub fn set_offset(&mut self, offset_mm: f64, now_ms: u64) {
        self.record(now_ms);
        self.offset_mm = Some(offset_mm);
        self.stats = None;
    }

    #[allow(dead_code)] // used by the 3D preview and section features in phase 3
    pub fn set_custom(&mut self, normal: Vec3, offset_mm: Option<f64>, now_ms: u64) {
        self.record(now_ms);
        self.custom_normal = normal;
        self.offset_mm = offset_mm;
        self.stats = None;
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
        self.offset_mm.unwrap_or_else(|| {
            let c = bounds.center();
            match self.plane {
                PlaneKind::XY => c[2],
                PlaneKind::XZ => c[1],
                PlaneKind::YZ => c[0],
                PlaneKind::Custom => norm(self.custom_normal).map_or(0.0, |n| dot(n, c)),
            }
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
