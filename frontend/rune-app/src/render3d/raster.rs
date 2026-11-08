//! CPU z-buffer rasterizer for the display mesh: per-vertex lighting, depth-tested feature
//! edges, an alpha-blended plane guide and an id buffer used for picking. Rows are split into
//! bands that run on scoped threads, so it needs no GPU path and no dependencies.
//!
//! shortcut: triangles/lines with a vertex behind the near plane are dropped, not clipped; upgrade
//! if holes show when the eye is inside the model.

use std::thread;

use super::{Camera, FOV_DEG, V3};

pub struct Body<'a> {
    pub id: u32,
    pub positions: &'a [f32],
    pub normals: &'a [f32],
    pub indices: &'a [u32],
    pub edges: &'a [u32],
    /// 0..255 base color and additive (selection) tint.
    pub color: [f32; 3],
    pub emissive: [f32; 3],
}

/// A world-space line; `on_top` skips the depth test.
pub struct Line {
    pub a: V3,
    pub b: V3,
    pub color: [u8; 3],
    pub on_top: bool,
}

pub struct Scene<'a> {
    pub bodies: Vec<Body<'a>>,
    pub lines: Vec<Line>,
    pub guide: Option<[V3; 4]>,
    pub background: [u8; 3],
    pub edge_color: [u8; 3],
}

#[derive(Clone, Copy, Default)]
struct Sv {
    x: f32,
    y: f32,
    /// 1 / view depth (0: behind the near plane); larger is closer and interpolates linearly.
    z: f32,
    shade: f32,
}

#[derive(Default)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    /// Body id + 1 per pixel, 0 for background.
    ids: Vec<u32>,
    depth: Vec<f32>,
    svs: Vec<Vec<Sv>>,
}

const EDGE_BIAS: f32 = 3e-4;
const GUIDE_FILL: [f32; 3] = [0x4b as f32, 0xb3 as f32, 0xfd as f32];
const GUIDE_ALPHA: f32 = 0.2;
const GUIDE_EDGE: [u8; 3] = [0x1a, 0x5f, 0xb4];

fn threads() -> usize {
    thread::available_parallelism().map_or(1, |n| n.get()).min(16)
}

/// Runs `f(offset, chunk)` over `out` split across threads (inline when small).
fn par_chunks<T: Send>(out: &mut [T], f: impl Fn(usize, &mut [T]) + Sync) {
    let n = threads();
    if out.len() < 32_768 || n == 1 {
        return f(0, out);
    }
    let chunk = out.len().div_ceil(n);
    thread::scope(|s| {
        for (i, c) in out.chunks_mut(chunk).enumerate() {
            let f = &f;
            s.spawn(move || f(i * chunk, c));
        }
    });
}

struct Proj {
    eye: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    fwd: [f32; 3],
    fx: f32,
    fy: f32,
    cx: f32,
    cy: f32,
    near: f32,
}

impl Proj {
    fn new(cam: &Camera, w: usize, h: usize) -> Proj {
        let (r, u, f) = cam.basis();
        let to32 = |v: V3| v.map(|c| c as f32);
        let focal = 1.0 / (FOV_DEG.to_radians() / 2.0).tan() as f32;
        Proj {
            eye: to32(cam.eye()),
            right: to32(r),
            up: to32(u),
            fwd: to32(f),
            fx: focal / (w as f32 / h as f32) * w as f32 / 2.0,
            fy: focal * h as f32 / 2.0,
            cx: w as f32 / 2.0,
            cy: h as f32 / 2.0,
            near: (cam.distance * 0.01).max(1e-4) as f32,
        }
    }

    fn point(&self, p: [f32; 3], shade: f32) -> Sv {
        let d = [p[0] - self.eye[0], p[1] - self.eye[1], p[2] - self.eye[2]];
        let dot = |a: [f32; 3]| d[0] * a[0] + d[1] * a[1] + d[2] * a[2];
        let depth = dot(self.fwd);
        if depth < self.near {
            return Sv::default();
        }
        let inv = 1.0 / depth;
        Sv {
            x: self.cx + dot(self.right) * inv * self.fx,
            y: self.cy - dot(self.up) * inv * self.fy,
            z: inv,
            shade,
        }
    }
}

/// Fixed world light like the Three.js scene's, two-sided.
fn shade(n: &[f32]) -> f32 {
    const L: [f32; 3] = [0.408_248_3, -0.408_248_3, 0.816_496_6];
    0.5 + 0.5 * (n[0] * L[0] + n[1] * L[1] + n[2] * L[2]).abs().min(1.0)
}

/// Calls `px(x, y, z, shade)` for every pixel center inside the triangle within rows `y0..y1`.
fn fill(a: Sv, mut b: Sv, mut c: Sv, y0: usize, y1: usize, w: usize, mut px: impl FnMut(usize, usize, f32, f32)) {
    if a.z <= 0.0 || b.z <= 0.0 || c.z <= 0.0 {
        return;
    }
    let (min_y, max_y) = (a.y.min(b.y).min(c.y), a.y.max(b.y).max(c.y));
    if max_y < y0 as f32 || min_y >= y1 as f32 {
        return;
    }
    let (min_x, max_x) = (a.x.min(b.x).min(c.x), a.x.max(b.x).max(c.x));
    if max_x < 0.0 || min_x >= w as f32 {
        return;
    }
    let mut area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if area == 0.0 || !area.is_finite() {
        return;
    }
    if area < 0.0 {
        std::mem::swap(&mut b, &mut c);
        area = -area;
    }
    let inv = 1.0 / area;
    let xs = ((min_x - 0.5).ceil().max(0.0)) as usize;
    let xe = ((max_x - 0.5).floor().min(w as f32 - 1.0)).max(-1.0) as i64;
    let ys = ((min_y - 0.5).ceil().max(y0 as f32)) as usize;
    let ye = ((max_y - 0.5).floor().min(y1 as f32 - 1.0)).max(-1.0) as i64;
    if xe < xs as i64 || ye < ys as i64 {
        return;
    }
    // Edge function of the edge opposite each vertex, stepped per pixel.
    let (d0x, d1x, d2x) = (b.y - c.y, c.y - a.y, a.y - b.y);
    for y in ys..=ye as usize {
        let (sx, sy) = (xs as f32 + 0.5, y as f32 + 0.5);
        let mut e0 = (c.x - b.x) * (sy - b.y) - (c.y - b.y) * (sx - b.x);
        let mut e1 = (a.x - c.x) * (sy - c.y) - (a.y - c.y) * (sx - c.x);
        let mut e2 = (b.x - a.x) * (sy - a.y) - (b.y - a.y) * (sx - a.x);
        let mut inside = false;
        for x in xs..=xe as usize {
            if e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0 {
                inside = true;
                let (l0, l1, l2) = (e0 * inv, e1 * inv, e2 * inv);
                px(x, y, l0 * a.z + l1 * b.z + l2 * c.z, l0 * a.shade + l1 * b.shade + l2 * c.shade);
            } else if inside {
                break;
            }
            e0 += d0x;
            e1 += d1x;
            e2 += d2x;
        }
    }
}

/// Calls `px(x, y, z)` along the segment clipped to rows `y0..y1` and the width.
fn line(a: Sv, b: Sv, y0: usize, y1: usize, w: usize, mut px: impl FnMut(usize, usize, f32)) {
    if a.z <= 0.0 || b.z <= 0.0 {
        return;
    }
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [
        (-dx, a.x),
        (dx, w as f32 - a.x),
        (-dy, a.y - y0 as f32),
        (dy, y1 as f32 - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    if t0 > t1 || !t0.is_finite() || !t1.is_finite() {
        return;
    }
    let steps = ((dx.abs().max(dy.abs())) * (t1 - t0)).ceil().max(1.0) as usize;
    for i in 0..=steps {
        let t = t0 + (t1 - t0) * i as f32 / steps as f32;
        let x = ((a.x + dx * t) as usize).min(w - 1);
        let y = ((a.y + dy * t) as usize).clamp(y0, y1 - 1);
        px(x, y, a.z + (b.z - a.z) * t);
    }
}

struct Band<'a> {
    y0: usize,
    y1: usize,
    rgba: &'a mut [u8],
    depth: &'a mut [f32],
    ids: &'a mut [u32],
}

impl Frame {
    /// Renders `scene` and returns top-first RGBA pixels; the id buffer is kept for `pick`.
    pub fn render(&mut self, width: usize, height: usize, cam: &Camera, scene: &Scene) -> Vec<u8> {
        let (width, height) = (width.max(1), height.max(1));
        (self.width, self.height) = (width, height);
        let proj = Proj::new(cam, width, height);
        self.ids.resize(width * height, 0);
        self.depth.resize(width * height, 0.0);
        self.svs.resize_with(scene.bodies.len(), Vec::new);
        for (body, out) in scene.bodies.iter().zip(&mut self.svs) {
            out.resize(body.positions.len() / 3, Sv::default());
            par_chunks(out, |first, chunk| {
                for (k, sv) in chunk.iter_mut().enumerate() {
                    let i = (first + k) * 3;
                    *sv = proj.point([body.positions[i], body.positions[i + 1], body.positions[i + 2]], shade(&body.normals[i..i + 3]));
                }
            });
        }
        let lines: Vec<(Sv, Sv, [u8; 3], bool)> = scene
            .lines
            .iter()
            .map(|l| (proj.point(l.a.map(|v| v as f32), 1.0), proj.point(l.b.map(|v| v as f32), 1.0), l.color, l.on_top))
            .collect();
        let guide = scene.guide.map(|g| g.map(|c| proj.point(c.map(|v| v as f32), 1.0)));

        let mut rgba = vec![0u8; width * height * 4];
        let rows = height.div_ceil(threads());
        let svs = &self.svs;
        thread::scope(|s| {
            let bands = rgba
                .chunks_mut(rows * width * 4)
                .zip(self.depth.chunks_mut(rows * width))
                .zip(self.ids.chunks_mut(rows * width));
            for (i, ((rgba, depth), ids)) in bands.enumerate() {
                let (lines, guide) = (&lines, &guide);
                s.spawn(move || {
                    let y0 = i * rows;
                    let band = Band { y0, y1: (y0 + rows).min(height), rgba, depth, ids };
                    draw_band(band, width, scene, svs, lines, guide);
                });
            }
        });
        rgba
    }

    /// The body id drawn at pixel (x, y) of the last render.
    pub fn pick(&self, x: usize, y: usize) -> Option<u32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.ids[y * self.width + x].checked_sub(1)
    }
}

fn draw_band(band: Band, w: usize, scene: &Scene, svs: &[Vec<Sv>], lines: &[(Sv, Sv, [u8; 3], bool)], guide: &Option<[Sv; 4]>) {
    let Band { y0, y1, rgba, depth, ids } = band;
    let bg = scene.background;
    for p in rgba.chunks_exact_mut(4) {
        p.copy_from_slice(&[bg[0], bg[1], bg[2], 255]);
    }
    depth.fill(0.0);
    ids.fill(0);
    for (body, sv) in scene.bodies.iter().zip(svs) {
        let (base, em) = (body.color, body.emissive);
        for t in body.indices.chunks_exact(3) {
            fill(sv[t[0] as usize], sv[t[1] as usize], sv[t[2] as usize], y0, y1, w, |x, y, z, sh| {
                let i = (y - y0) * w + x;
                if z > depth[i] {
                    depth[i] = z;
                    ids[i] = body.id + 1;
                    for k in 0..3 {
                        rgba[i * 4 + k] = (base[k] * sh + em[k]).clamp(0.0, 255.0) as u8;
                    }
                }
            });
        }
    }
    if let Some([a, b, c, d]) = *guide {
        for (p, q, r) in [(a, b, c), (a, c, d)] {
            fill(p, q, r, y0, y1, w, |x, y, z, _| {
                let i = (y - y0) * w + x;
                if z >= depth[i] {
                    for k in 0..3 {
                        let old = f32::from(rgba[i * 4 + k]);
                        rgba[i * 4 + k] = (old + (GUIDE_FILL[k] - old) * GUIDE_ALPHA) as u8;
                    }
                }
            });
        }
    }
    let mut put = |x: usize, y: usize, z: f32, color: [u8; 3], on_top: bool| {
        let i = (y - y0) * w + x;
        if on_top || z * (1.0 + EDGE_BIAS) >= depth[i] {
            rgba[i * 4..i * 4 + 3].copy_from_slice(&color);
        }
    };
    for (body, sv) in scene.bodies.iter().zip(svs) {
        for e in body.edges.chunks_exact(2) {
            line(sv[e[0] as usize], sv[e[1] as usize], y0, y1, w, |x, y, z| put(x, y, z, scene.edge_color, false));
        }
    }
    for &(a, b, color, on_top) in lines {
        line(a, b, y0, y1, w, |x, y, z| put(x, y, z, color, on_top));
    }
}

/// The guide outline, drawn over the model.
pub fn guide_outline(corners: [V3; 4]) -> Vec<Line> {
    (0..4)
        .map(|i| Line { a: corners[i], b: corners[(i + 1) % 4], color: GUIDE_EDGE, on_top: true })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::render3d::StandardView;

    pub(crate) type Mesh = (Vec<f32>, Vec<f32>, Vec<u32>, Vec<u32>);

    /// Unit cube [0,s]^3: 8 shared vertices, 12 triangles, 12 edges.
    pub(crate) fn cube(s: f32, at: [f32; 3]) -> Mesh {
        let mut pos = vec![];
        for i in 0..8u32 {
            pos.extend([at[0] + s * (i & 1) as f32, at[1] + s * (i >> 1 & 1) as f32, at[2] + s * (i >> 2 & 1) as f32]);
        }
        let normals = pos.iter().map(|_| 0.577).collect();
        let quads = [[0, 1, 3, 2], [4, 5, 7, 6], [0, 1, 5, 4], [2, 3, 7, 6], [0, 2, 6, 4], [1, 3, 7, 5]];
        let idx = quads.iter().flat_map(|q| [q[0], q[1], q[2], q[0], q[2], q[3]]).collect();
        let edges = quads.iter().flat_map(|q| [q[0], q[1], q[1], q[2], q[2], q[3], q[3], q[0]]).collect();
        (pos, normals, idx, edges)
    }

    fn scene<'a>(cubes: &'a [Mesh]) -> Scene<'a> {
        Scene {
            bodies: cubes
                .iter()
                .enumerate()
                .map(|(i, c)| Body {
                    id: i as u32,
                    positions: &c.0,
                    normals: &c.1,
                    indices: &c.2,
                    edges: &c.3,
                    color: [200.0, 100.0, 50.0],
                    emissive: [0.0; 3],
                })
                .collect(),
            lines: vec![],
            guide: None,
            background: [10, 20, 30],
            edge_color: [0, 0, 0],
        }
    }

    #[test]
    fn a_cube_in_the_center_is_drawn_and_picked_and_the_corner_is_background() {
        let cubes = [cube(2.0, [-1.0, -1.0, -1.0])];
        let cam = Camera::looking([0.0; 3], StandardView::Front.direction(), 1.8, 1.0);
        let mut frame = Frame::default();
        let rgba = frame.render(200, 200, &cam, &scene(&cubes));
        assert_eq!(rgba.len(), 200 * 200 * 4);
        assert_eq!(frame.pick(100, 100), Some(0));
        assert_eq!(frame.pick(2, 2), None);
        assert_eq!(&rgba[..4], &[10, 20, 30, 255]);
        let c = &rgba[(100 * 200 + 100) * 4..][..3];
        assert!(c[0] > c[1] && c[1] > c[2], "body color, not background: {c:?}");
        assert_eq!(frame.pick(500, 0), None);
    }

    #[test]
    fn the_nearer_body_wins_and_hidden_bodies_are_absent() {
        // Front view looks along +Y: the cube at smaller Y is nearer.
        let cubes = [cube(2.0, [-1.0, 0.5, -1.0]), cube(2.0, [-1.0, -2.5, -1.0])];
        let cam = Camera::looking([0.0, -0.5, 0.0], StandardView::Front.direction(), 3.0, 1.0);
        let mut frame = Frame::default();
        let mut s = scene(&cubes);
        frame.render(120, 120, &cam, &s);
        assert_eq!(frame.pick(60, 60), Some(1));
        s.bodies.remove(1);
        frame.render(120, 120, &cam, &s);
        assert_eq!(frame.pick(60, 60), Some(0));
    }

    #[test]
    fn edges_and_the_plane_guide_are_visible_and_the_guide_tints_the_model() {
        let cubes = [cube(2.0, [-1.0, -1.0, -1.0])];
        let cam = Camera::looking([0.0; 3], StandardView::Front.direction(), 1.8, 1.0);
        let mut s = scene(&cubes);
        let mut frame = Frame::default();
        let plain = frame.render(200, 200, &cam, &s);
        let dark = plain.chunks_exact(4).filter(|p| p[..3] == [0, 0, 0]).count();
        assert!(dark > 100, "feature edges drawn: {dark}");
        s.guide = Some([[-1.1, -2.0, -1.1], [1.1, -2.0, -1.1], [1.1, -2.0, 1.1], [-1.1, -2.0, 1.1]]);
        s.lines = guide_outline(s.guide.unwrap());
        let guided = frame.render(200, 200, &cam, &s);
        assert_ne!(plain[(100 * 200 + 100) * 4 + 2], guided[(100 * 200 + 100) * 4 + 2]);
        assert!(guided.chunks_exact(4).any(|p| p[..3] == GUIDE_EDGE));
    }

    /// UV sphere shell with `n` x `n` quads (2 n^2 triangles) around `at`.
    fn sphere(n: usize, r: f32, at: [f32; 3]) -> Mesh {
        let (mut pos, mut nor, mut idx, mut edges) = (vec![], vec![], vec![], vec![]);
        for i in 0..=n {
            let th = std::f32::consts::PI * i as f32 / n as f32;
            for j in 0..=n {
                let ph = std::f32::consts::TAU * j as f32 / n as f32;
                let d = [th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos()];
                pos.extend([at[0] + r * d[0], at[1] + r * d[1], at[2] + r * d[2]]);
                nor.extend(d);
            }
        }
        let w = (n + 1) as u32;
        for i in 0..n as u32 {
            for j in 0..n as u32 {
                let a = i * w + j;
                idx.extend([a, a + 1, a + w, a + 1, a + w + 1, a + w]);
                if i % 8 == 0 {
                    edges.extend([a, a + 1]);
                }
            }
        }
        (pos, nor, idx, edges)
    }

    /// `cargo test --release bench_ -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_frame_times_and_pick_latency() {
        for (label, bodies, n) in [("small 20k", 20, 22), ("typical 200k", 50, 45), ("large 1M", 100, 71), ("huge 2M", 100, 100)] {
            let meshes: Vec<_> = (0..bodies).map(|b| sphere(n, 1.0, [(b % 10) as f32 * 1.5, (b / 10) as f32 * 1.5, 0.0])).collect();
            let tris: usize = meshes.iter().map(|m| m.2.len() / 3).sum();
            let mut s = scene(&meshes);
            s.lines = vec![Line { a: [0.0; 3], b: [15.0, 15.0, 0.0], color: [9, 9, 9], on_top: false }];
            let cam = Camera::looking([7.0, 7.0, 0.0], StandardView::Iso.direction(), 11.0, 1.7);
            for (w, h) in [(1280, 760), (2560, 1520)] {
                let mut frame = Frame::default();
                frame.render(w, h, &cam, &s);
                let t = std::time::Instant::now();
                for _ in 0..5 {
                    frame.render(w, h, &cam, &s);
                }
                let ms = t.elapsed().as_secs_f64() * 200.0;
                let t = std::time::Instant::now();
                let hit = (0..1000).filter(|i| frame.pick(i % w, i % h).is_some()).count();
                let pick_us = t.elapsed().as_secs_f64() * 1e3;
                let mb = (w * h * (4 + 4 + 4)) as f64 / 1e6;
                println!("{label:>13} {tris:>8} tris {w}x{h}: {ms:7.1} ms/frame, pick {pick_us:.2} us ({hit}), buffers {mb:.0} MB, threads {}", threads());
            }
        }
    }
}
