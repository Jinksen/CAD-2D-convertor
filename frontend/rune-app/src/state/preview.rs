//! Viewport state shared by the 2D drawing and 3D preview: mode, display mesh, orbit camera.
//! The mesh is display-only and never reaches sectioning or export.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::sync::Arc;

use rune::Pixels;

use crate::api::dto::{BoundsMm, PreviewMesh};
use crate::render3d::raster::Frame;
use crate::render3d::{Camera, PlaneGuide, StandardView, bounds_sphere};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewportMode {
    #[default]
    TwoD,
    ThreeD,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum MeshStatus {
    #[default]
    Empty,
    Loading,
    Ready,
    Failed(String),
}

/// Everything a rendered frame depends on; an unchanged key means the last frame is current
/// (the counterpart of the React render scheduler: draw on demand only).
#[derive(Debug, Clone, PartialEq)]
pub struct RenderKey {
    pub size: (usize, usize),
    pub camera: Camera,
    pub hidden: BTreeSet<String>,
    pub selected: Option<String>,
    pub guide: Option<PlaneGuide>,
    pub background: [u8; 3],
}

#[derive(Default)]
pub struct PreviewState {
    pub mode: ViewportMode,
    pub status: MeshStatus,
    pub mesh: Option<Arc<PreviewMesh>>,
    pub camera: Camera,
    /// Camera at the start of the current drag.
    pub drag_from: Camera,
    /// The viewport element in points, reported by its layout.
    pub viewport: (f32, f32),
    /// Device pixels per point, set by the root view from `Cx::env` (0 until then).
    pub scale: Cell<f32>,
    pub pixels: Pixels,
    /// Buffers and id buffer of the last render, used for picking.
    pub frame: RefCell<Frame>,
    pub drawn: RefCell<Option<RenderKey>>,
    /// Import id the mesh was requested for (one fetch per import).
    pub requested: Option<String>,
    pub last_frame_ms: Cell<f32>,
}

impl PreviewState {
    pub fn begin_loading(&mut self) {
        self.status = MeshStatus::Loading;
        self.mesh = None;
        *self.drawn.borrow_mut() = None;
    }

    pub fn finish(&mut self, result: Result<PreviewMesh, String>, bounds: &BoundsMm) {
        match result {
            Ok(mesh) => {
                self.mesh = Some(Arc::new(mesh));
                self.status = MeshStatus::Ready;
                self.view(StandardView::Iso, bounds);
            }
            Err(message) => self.status = MeshStatus::Failed(message),
        }
    }

    fn aspect(&self) -> f64 {
        let (w, h) = self.viewport;
        if w > 0.0 && h > 0.0 {
            f64::from(w / h)
        } else {
            1.0
        }
    }

    /// Standard view fitted to the whole model.
    pub fn view(&mut self, view: StandardView, bounds: &BoundsMm) {
        let (center, radius) = bounds_sphere(bounds.min_xyz, bounds.max_xyz);
        self.camera = Camera::looking(center, view.direction(), radius, self.aspect());
    }

    /// Fits the selected body (all when `None`), keeping the viewing direction.
    pub fn fit(&mut self, body: Option<&str>, bounds: &BoundsMm) {
        let (min, max) = body
            .and_then(|id| {
                let mesh = self.mesh.as_ref()?;
                positions_box(&mesh.components.iter().find(|c| c.component_id == id)?.positions)
            })
            .unwrap_or((bounds.min_xyz, bounds.max_xyz));
        let (center, radius) = bounds_sphere(min, max);
        self.camera.fit(center, radius, self.aspect());
    }
}

fn positions_box(positions: &[f32]) -> Option<([f64; 3], [f64; 3])> {
    let mut points = positions.chunks_exact(3);
    let first = points.next()?;
    let mut lo = [first[0], first[1], first[2]].map(f64::from);
    let mut hi = lo;
    for p in points {
        for k in 0..3 {
            lo[k] = lo[k].min(f64::from(p[k]));
            hi[k] = hi[k].max(f64::from(p[k]));
        }
    }
    Some((lo, hi))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::dto::PreviewComponentMesh;

    fn mesh() -> PreviewMesh {
        PreviewMesh {
            import_id: "imp".into(),
            components: vec![PreviewComponentMesh {
                component_id: "a".into(),
                color: None,
                positions: vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 2.0],
                normals: vec![0.0; 9],
                indices: vec![0, 1, 2],
                edge_indices: vec![],
            }],
            deflection_mm: 0.1,
            coarsened: false,
        }
    }

    const BOUNDS: BoundsMm = BoundsMm {
        min_xyz: [0.0; 3],
        max_xyz: [10.0, 10.0, 10.0],
    };

    #[test]
    fn a_loaded_mesh_is_fitted_and_a_selected_body_refits_to_its_own_box() {
        let mut p = PreviewState::default();
        p.begin_loading();
        assert_eq!(p.status, MeshStatus::Loading);
        p.finish(Ok(mesh()), &BOUNDS);
        assert_eq!(p.status, MeshStatus::Ready);
        assert_eq!(p.camera.target, [5.0, 5.0, 5.0]);
        let whole = p.camera.distance;
        p.fit(Some("a"), &BOUNDS);
        assert_eq!(p.camera.target, [1.0, 1.0, 1.0]);
        assert!(p.camera.distance < whole);
        p.fit(Some("missing"), &BOUNDS);
        assert_eq!(p.camera.target, [5.0, 5.0, 5.0]);
    }

    #[test]
    fn a_failed_load_keeps_the_message() {
        let mut p = PreviewState::default();
        p.finish(Err("boom".into()), &BOUNDS);
        assert_eq!(p.status, MeshStatus::Failed("boom".into()));
        assert!(p.mesh.is_none());
    }
}
