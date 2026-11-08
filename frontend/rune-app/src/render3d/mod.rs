//! 3D preview: orbit camera math, standard views and the section-plane guide (ports of
//! `cameraViews.ts` and `sectionPlane.ts`), plus a CPU rasterizer in `raster`. Display-only.

pub mod raster;

use crate::api::dto::{BoundsMm, SectionPlane};

pub type V3 = [f64; 3];

pub const FOV_DEG: f64 = 45.0;
const MAX_PITCH: f64 = std::f64::consts::FRAC_PI_2 - 1e-3;

fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn scale(a: V3, k: f64) -> V3 {
    a.map(|v| v * k)
}
fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardView {
    Iso,
    Top,
    Front,
    Right,
}

impl StandardView {
    pub const ALL: [StandardView; 4] = [Self::Iso, Self::Top, Self::Front, Self::Right];

    pub fn label(self) -> &'static str {
        match self {
            Self::Iso => "Iso",
            Self::Top => "Top",
            Self::Front => "Front",
            Self::Right => "Right",
        }
    }

    /// Direction from the target to the camera; the model's +Z stays up on screen.
    pub fn direction(self) -> V3 {
        match self {
            Self::Iso => [1.0, -1.0, 0.75],
            Self::Top => [0.0, -1e-6, 1.0],
            Self::Front => [0.0, -1.0, 0.0],
            Self::Right => [1.0, 0.0, 0.0],
        }
    }
}

/// Center and enclosing-sphere radius of an axis-aligned box.
pub fn bounds_sphere(min: V3, max: V3) -> (V3, f64) {
    (
        std::array::from_fn(|i| (min[i] + max[i]) / 2.0),
        len(sub(max, min)) / 2.0,
    )
}

/// Camera distance at which a sphere of `radius` fits the narrower field of view.
pub fn fit_distance(radius: f64, vertical_fov_deg: f64, aspect: f64) -> f64 {
    let vertical = vertical_fov_deg.to_radians();
    let horizontal = 2.0 * ((vertical / 2.0).tan() * aspect).atan();
    radius.max(1e-6) / (vertical.min(horizontal) / 2.0).sin() * 1.05
}

/// Z-up orbit camera: `yaw` is the azimuth of the eye around the target, `pitch` its elevation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub target: V3,
    pub yaw: f64,
    pub pitch: f64,
    pub distance: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::looking([0.0; 3], StandardView::Iso.direction(), 1.0, 1.0)
    }
}

impl Camera {
    /// Looks at the sphere (`center`, `radius`) from `direction`, fitted to the view.
    pub fn looking(center: V3, direction: V3, radius: f64, aspect: f64) -> Camera {
        let flat = direction[0].hypot(direction[1]);
        Camera {
            target: center,
            yaw: direction[1].atan2(direction[0]),
            pitch: direction[2].atan2(flat).clamp(-MAX_PITCH, MAX_PITCH),
            distance: fit_distance(radius, FOV_DEG, aspect),
        }
    }

    /// Same direction, refitted onto a new sphere (the "Fit" button).
    pub fn fit(&mut self, center: V3, radius: f64, aspect: f64) {
        self.target = center;
        self.distance = fit_distance(radius, FOV_DEG, aspect);
    }

    pub fn eye(&self) -> V3 {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        add(self.target, scale([cp * cy, cp * sy, sp], self.distance))
    }

    /// Screen right, screen up and viewing direction.
    pub fn basis(&self) -> (V3, V3, V3) {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        let forward = [-cp * cy, -cp * sy, -sp];
        let right = [-sy, cy, 0.0];
        (right, cross(right, forward), forward)
    }

    /// Left drag: a full viewport height is one turn, like OrbitControls.
    pub fn orbit(&mut self, dx: f64, dy: f64, height: f64) {
        let k = std::f64::consts::TAU / height.max(1.0);
        self.yaw -= dx * k;
        self.pitch = (self.pitch + dy * k).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Moves the target so the model follows the pointer.
    pub fn pan(&mut self, dx: f64, dy: f64, height: f64) {
        let k = 2.0 * self.distance * (FOV_DEG.to_radians() / 2.0).tan() / height.max(1.0);
        let (right, up, _) = self.basis();
        self.target = add(self.target, add(scale(right, -dx * k), scale(up, dy * k)));
    }

    /// Positive `amount` (wheel up, in points) moves closer.
    pub fn zoom(&mut self, amount: f64) {
        self.distance = (self.distance * (-amount * 0.0015).exp()).clamp(1e-6, 1e9);
    }
}

/// The translucent guide quad for the current section plane: `center` plus/minus half of the
/// edge vectors `u` and `v`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneGuide {
    pub center: V3,
    pub u: V3,
    pub v: V3,
}

impl PlaneGuide {
    pub fn corners(&self) -> [V3; 4] {
        let (u, v) = (scale(self.u, 0.5), scale(self.v, 0.5));
        [
            sub(sub(self.center, u), v),
            sub(add(self.center, u), v),
            add(add(self.center, u), v),
            add(sub(self.center, u), v),
        ]
    }
}

/// Port of `planeGuideFrame`: 8% larger than the model on each axis, or the model diagonal
/// for a custom plane (whose `origin_xyz` is `normal * offset`).
pub fn plane_guide(bounds: &BoundsMm, plane: &SectionPlane) -> PlaneGuide {
    let m = bounds.center();
    let span: V3 =
        std::array::from_fn(|i| ((bounds.max_xyz[i] - bounds.min_xyz[i]) * 1.08).max(1.0));
    match *plane {
        SectionPlane::XY { offset_mm } => PlaneGuide {
            center: [m[0], m[1], offset_mm],
            u: [span[0], 0.0, 0.0],
            v: [0.0, span[1], 0.0],
        },
        SectionPlane::XZ { offset_mm } => PlaneGuide {
            center: [m[0], offset_mm, m[2]],
            u: [span[0], 0.0, 0.0],
            v: [0.0, 0.0, span[2]],
        },
        SectionPlane::YZ { offset_mm } => PlaneGuide {
            center: [offset_mm, m[1], m[2]],
            u: [0.0, span[1], 0.0],
            v: [0.0, 0.0, span[2]],
        },
        SectionPlane::Custom {
            origin_xyz,
            normal_xyz: n,
            x_dir_xyz: x,
        } => {
            // Project the model midpoint onto the plane; size the guide to the model diagonal.
            let distance = dot(n, m) - dot(n, origin_xyz);
            let size = len(span).max(1.0);
            PlaneGuide {
                center: sub(m, scale(n, distance)),
                u: scale(x, size),
                v: scale(cross(n, x), size),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn fits_a_sphere_into_the_narrower_field_of_view() {
        assert!(near(
            fit_distance(1.0, 90.0, 1.0),
            std::f64::consts::SQRT_2 * 1.05
        ));
        assert!(fit_distance(1.0, 90.0, 0.5) > fit_distance(1.0, 90.0, 1.0));
        assert!(near(
            fit_distance(1.0, 90.0, 2.0),
            fit_distance(1.0, 90.0, 1.0)
        ));
    }

    #[test]
    fn bounds_sphere_is_the_box_center_and_half_diagonal() {
        let (c, r) = bounds_sphere([0.0, 0.0, 0.0], [2.0, 4.0, 4.0]);
        assert_eq!(c, [1.0, 2.0, 2.0]);
        assert!(near(r, 3.0));
    }

    #[test]
    fn standard_views_sit_on_the_expected_side_of_the_model() {
        let center = [10.0, 20.0, 30.0];
        let d = fit_distance(5.0, FOV_DEG, 1.0);
        let eye = |v: StandardView| Camera::looking(center, v.direction(), 5.0, 1.0).eye();
        let (top, front, right, iso) = (
            eye(StandardView::Top),
            eye(StandardView::Front),
            eye(StandardView::Right),
            eye(StandardView::Iso),
        );
        assert!((top[2] - 30.0 - d).abs() < 1e-3 && (top[0] - 10.0).abs() < 1e-3);
        assert!(near(front[1] - 20.0, -d));
        assert!(near(right[0] - 10.0, d));
        assert!(near(len(sub(iso, center)), d));
    }

    #[test]
    fn basis_is_orthonormal_and_keeps_z_up() {
        for view in StandardView::ALL {
            let (r, u, f) = Camera::looking([0.0; 3], view.direction(), 1.0, 1.0).basis();
            assert!(near(len(r), 1.0) && near(len(u), 1.0) && near(len(f), 1.0));
            assert!(dot(r, u).abs() < 1e-9 && dot(r, f).abs() < 1e-9);
            assert!(u[2] >= 0.0, "{view:?}");
        }
        let (r, ..) = Camera::looking([0.0; 3], StandardView::Front.direction(), 1.0, 1.0).basis();
        assert!(near(r[0], 1.0), "front view has +X to the right");
    }

    #[test]
    fn orbit_turns_once_per_viewport_height_and_pitch_is_clamped() {
        let mut c = Camera::default();
        let yaw = c.yaw;
        c.orbit(100.0, 0.0, 400.0);
        assert!(near(yaw - c.yaw, std::f64::consts::FRAC_PI_2));
        c.orbit(0.0, 1e6, 400.0);
        assert!(c.pitch <= MAX_PITCH);
        c.orbit(0.0, -1e7, 400.0);
        assert!(c.pitch >= -MAX_PITCH);
    }

    #[test]
    fn pan_moves_the_target_in_the_screen_plane_and_zoom_scales_distance() {
        let mut c = Camera::looking([0.0; 3], StandardView::Front.direction(), 1.0, 1.0);
        let before = c.distance;
        c.pan(-50.0, 0.0, 500.0);
        assert!(c.target[0] > 0.0 && near(c.target[1], 0.0) && near(c.target[2], 0.0));
        c.zoom(100.0);
        assert!(c.distance < before);
        c.zoom(-100.0);
        assert!(near(c.distance, before));
    }

    #[test]
    fn fit_keeps_direction_and_recenters() {
        let mut c = Camera::default();
        let (yaw, pitch) = (c.yaw, c.pitch);
        c.fit([5.0, 5.0, 5.0], 2.0, 1.0);
        assert_eq!((c.yaw, c.pitch, c.target), (yaw, pitch, [5.0, 5.0, 5.0]));
        assert!(near(c.distance, fit_distance(2.0, FOV_DEG, 1.0)));
    }

    const BOUNDS: BoundsMm = BoundsMm {
        min_xyz: [0.0, 10.0, 20.0],
        max_xyz: [100.0, 50.0, 80.0],
    };

    #[test]
    fn principal_guides_sit_at_the_offset_and_cover_the_extents() {
        let xy = plane_guide(&BOUNDS, &SectionPlane::XY { offset_mm: 33.0 });
        assert_eq!(xy.center, [50.0, 30.0, 33.0]);
        assert!(xy.u[0] > 100.0 && xy.v[1] > 40.0);
        assert_eq!(
            plane_guide(&BOUNDS, &SectionPlane::XZ { offset_mm: 25.0 }).center,
            [50.0, 25.0, 50.0]
        );
        let yz = plane_guide(&BOUNDS, &SectionPlane::YZ { offset_mm: 70.0 });
        assert_eq!(yz.center, [70.0, 30.0, 50.0]);
        assert!(near(yz.u[1], 43.2) && near(yz.v[2], 64.8));
    }

    #[test]
    fn a_custom_guide_lies_on_the_requested_plane() {
        let raw = [0.331, 0.013, 0.944];
        let n = scale(raw, 1.0 / len(raw));
        let r = [1.0 - n[0] * n[0], -n[0] * n[1], -n[0] * n[2]];
        let x = scale(r, 1.0 / len(r));
        let g = plane_guide(
            &BOUNDS,
            &SectionPlane::Custom {
                origin_xyz: scale(n, 60.0),
                normal_xyz: n,
                x_dir_xyz: x,
            },
        );
        assert!(near(dot(n, g.center), 60.0));
        assert!(near(dot(cross(g.u, g.v), n).signum(), 1.0));
        assert!(g.corners().iter().all(|c| (dot(n, *c) - 60.0).abs() < 1e-6));
    }
}
