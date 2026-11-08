//! Display-only geometry for the 2D view: bounds, sampling and tolerance-based tessellation of the
//! analytic section curves. Nothing here feeds back into the section data or the export.

use super::camera::ViewBox;
use crate::api::dto::{Point2, SectionCurve, SectionWire};

type P = [f64; 2];

struct Arc {
    center: P,
    x_axis: P,
    y_axis: P,
    major: f64,
    minor: f64,
    start: f64,
    end: f64,
}

impl Arc {
    fn point(&self, t: f64) -> P {
        let (cos, sin) = (t.cos(), t.sin());
        [
            self.center[0] + self.major * self.x_axis[0] * cos + self.minor * self.y_axis[0] * sin,
            self.center[1] + self.major * self.x_axis[1] * cos + self.minor * self.y_axis[1] * sin,
        ]
    }
}

/// The arc/ellipse parameters of a curve; `None` for lines.
fn arc(c: &SectionCurve) -> Option<Arc> {
    match *c {
        SectionCurve::Line { .. } => None,
        SectionCurve::Circle {
            center,
            radius,
            x_axis,
            y_axis,
            start_parameter,
            end_parameter,
        } => Some(Arc {
            center,
            x_axis,
            y_axis,
            major: radius,
            minor: radius,
            start: start_parameter,
            end: end_parameter,
        }),
        SectionCurve::Ellipse {
            center,
            major_radius,
            minor_radius,
            x_axis,
            y_axis,
            start_parameter,
            end_parameter,
        } => Some(Arc {
            center,
            x_axis,
            y_axis,
            major: major_radius,
            minor: minor_radius,
            start: start_parameter,
            end: end_parameter,
        }),
    }
}

pub fn start_point(c: &SectionCurve) -> Point2 {
    match c {
        SectionCurve::Line { start, .. } => *start,
        _ => arc(c).map_or([0.0; 2], |a| a.point(a.start)),
    }
}

pub fn end_point(c: &SectionCurve) -> Point2 {
    match c {
        SectionCurve::Line { end, .. } => *end,
        _ => arc(c).map_or([0.0; 2], |a| a.point(a.end)),
    }
}

/// Conservative box (`[min_x, min_y, max_x, max_y]`) of a curve, cheap enough to cull 100k curves.
pub fn curve_box(c: &SectionCurve) -> [f64; 4] {
    match (c, arc(c)) {
        (SectionCurve::Line { start, end }, _) => [
            start[0].min(end[0]),
            start[1].min(end[1]),
            start[0].max(end[0]),
            start[1].max(end[1]),
        ],
        (_, Some(a)) => {
            let r = a.major.max(a.minor);
            [
                a.center[0] - r,
                a.center[1] - r,
                a.center[0] + r,
                a.center[1] + r,
            ]
        }
        _ => [0.0; 4],
    }
}

/// Bounds (y up) of a curve set: lines by endpoints, arcs by 33 samples over the swept range.
pub fn bounds<'a>(curves: impl IntoIterator<Item = &'a SectionCurve>) -> Option<ViewBox> {
    let mut b: Option<[f64; 4]> = None;
    let mut add = |p: P| {
        let r = b.get_or_insert([p[0], p[1], p[0], p[1]]);
        r[0] = r[0].min(p[0]);
        r[1] = r[1].min(p[1]);
        r[2] = r[2].max(p[0]);
        r[3] = r[3].max(p[1]);
    };
    for c in curves {
        match arc(c) {
            None => {
                add(start_point(c));
                add(end_point(c));
            }
            Some(a) => {
                for k in 0..=32 {
                    add(a.point(a.start + (a.end - a.start) * f64::from(k) / 32.0));
                }
            }
        }
    }
    b.map(|b| ViewBox {
        x: b[0],
        y: b[1],
        width: b[2] - b[0],
        height: b[3] - b[1],
    })
}

/// Appends the polyline of `c` to `out`, within `tol` (section units) of the true curve. Lines
/// give their two endpoints; the first point is skipped when `skip_first` (a chained curve).
pub fn tessellate(c: &SectionCurve, tol: f64, skip_first: bool, out: &mut Vec<P>) {
    let Some(a) = arc(c) else {
        if let SectionCurve::Line { start, end } = c {
            if !skip_first {
                out.push(*start);
            }
            out.push(*end);
        }
        return;
    };
    let delta = a.end - a.start;
    let r = a.major.max(a.minor);
    // Chord sagitta r(1 - cos(h/2)) <= tol, never coarser than a quarter turn per segment.
    let step = if tol >= r {
        std::f64::consts::FRAC_PI_2
    } else {
        (2.0 * (1.0 - tol / r).acos()).min(std::f64::consts::FRAC_PI_2)
    };
    let n = (delta.abs() / step.max(1e-3)).ceil().clamp(1.0, 4096.0) as u32;
    for k in u32::from(skip_first)..=n {
        out.push(a.point(a.start + delta * f64::from(k) / f64::from(n)));
    }
}

fn same(a: P, b: P) -> bool {
    (a[0] - b[0]).abs() <= 1e-6 && (a[1] - b[1]).abs() <= 1e-6
}

/// Polylines of a wire: curves chain while each starts where the last ended; a gap starts a new
/// polyline so a real gap is drawn as a gap (never bridged). The flag is true when the wire is one
/// chain that returns to its start (the duplicate end point is dropped).
pub fn wire_polylines(wire: &SectionWire, tol: f64) -> (Vec<Vec<P>>, bool) {
    let mut lines: Vec<Vec<P>> = vec![];
    let mut cur: Vec<P> = vec![];
    for c in &wire.curves {
        let joined = cur.last().is_some_and(|l| same(*l, start_point(c)));
        if !joined && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
        }
        tessellate(c, tol, joined, &mut cur);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    let closed = wire.closed
        && lines.len() == 1
        && lines[0].len() > 2
        && lines[0]
            .first()
            .zip(lines[0].last())
            .is_some_and(|(f, l)| same(*f, *l));
    if closed {
        lines[0].pop();
    }
    (lines, closed)
}

/// Even-odd point-in-polygon.
pub fn inside(poly: &[P], p: P) -> bool {
    let mut odd = false;
    let mut j = poly.len().wrapping_sub(1);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            odd = !odd;
        }
        j = i;
    }
    odd
}

/// Distance from `p` to the segment `a`-`b`.
pub fn segment_distance(a: P, b: P, p: P) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}

/// Outline color legible on the canvas: on a light canvas colors lighter than mid-gray are
/// darkened in proportion; on a dark one dark colors are lightened. Fills keep the source color.
pub fn outline_color(rgb: [u8; 3], dark: bool) -> [u8; 3] {
    let [r, g, b] = rgb.map(f64::from);
    let lum = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0;
    let scale = if dark {
        if lum < 0.45 { 0.6 / lum.max(0.05) } else { 1.0 }
    } else if lum > 0.55 {
        0.38 / lum
    } else {
        1.0
    };
    rgb.map(|c| (f64::from(c) * scale).round().clamp(0.0, 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle(radius: f64, start: f64, end: f64) -> SectionCurve {
        SectionCurve::Circle {
            center: [0.0, 0.0],
            radius,
            x_axis: [1.0, 0.0],
            y_axis: [0.0, 1.0],
            start_parameter: start,
            end_parameter: end,
        }
    }

    #[test]
    fn a_full_circle_stays_within_tolerance_and_closes_on_itself() {
        let mut pts = vec![];
        tessellate(
            &circle(10.0, 0.0, std::f64::consts::TAU),
            0.01,
            false,
            &mut pts,
        );
        assert!(pts.len() > 40);
        let (first, last) = (pts[0], pts[pts.len() - 1]);
        assert!((first[0] - last[0]).abs() < 1e-9 && (first[1] - last[1]).abs() < 1e-9);
        let worst = pts
            .windows(2)
            .map(|w| 10.0 - ((w[0][0] + w[1][0]) / 2.0).hypot((w[0][1] + w[1][1]) / 2.0))
            .fold(0.0, f64::max);
        assert!(worst <= 0.0101, "sagitta {worst}");
    }

    #[test]
    fn coarser_tolerance_gives_fewer_points_and_arcs_keep_their_sweep() {
        let (mut fine, mut coarse) = (vec![], vec![]);
        let arc = circle(5.0, 0.0, std::f64::consts::FRAC_PI_2);
        tessellate(&arc, 0.001, false, &mut fine);
        tessellate(&arc, 1.0, false, &mut coarse);
        assert!(fine.len() > coarse.len() && coarse.len() >= 2);
        assert_eq!(start_point(&arc), [5.0, 0.0]);
        let e = end_point(&arc);
        assert!(e[0].abs() < 1e-9 && (e[1] - 5.0).abs() < 1e-9);
    }

    #[test]
    fn bounds_cover_arcs_and_lines() {
        let b = bounds(&[circle(2.0, 0.0, std::f64::consts::TAU)]).unwrap();
        assert!((b.x + 2.0).abs() < 1e-9 && (b.width - 4.0).abs() < 1e-9);
        assert!(bounds(&[]).is_none());
    }

    #[test]
    fn a_gap_between_curves_is_not_bridged() {
        let line = |a: P, b: P| SectionCurve::Line { start: a, end: b };
        let wire = SectionWire {
            closed: false,
            role: None,
            curves: vec![line([0.0, 0.0], [1.0, 0.0]), line([2.0, 0.0], [3.0, 0.0])],
        };
        let (lines, closed) = wire_polylines(&wire, 0.1);
        assert_eq!(lines.len(), 2);
        assert!(!closed);
        let square = SectionWire {
            closed: true,
            role: None,
            curves: vec![
                line([0.0, 0.0], [1.0, 0.0]),
                line([1.0, 0.0], [1.0, 1.0]),
                line([1.0, 1.0], [0.0, 1.0]),
                line([0.0, 1.0], [0.0, 0.0]),
            ],
        };
        let (lines, closed) = wire_polylines(&square, 0.1);
        assert!(closed && lines[0].len() == 4);
        assert!(inside(&lines[0], [0.5, 0.5]) && !inside(&lines[0], [1.5, 0.5]));
        assert_eq!(segment_distance([0.0, 0.0], [2.0, 0.0], [1.0, 3.0]), 3.0);
    }

    #[test]
    fn light_colors_are_darkened_on_a_light_canvas_and_dark_ones_lifted_on_a_dark_one() {
        assert_eq!(outline_color([255, 255, 255], false), [97, 97, 97]);
        assert_eq!(outline_color([20, 40, 80], false), [20, 40, 80]);
        assert!(outline_color([10, 10, 10], true)[0] > 100);
        assert_eq!(outline_color([200, 200, 200], true), [200, 200, 200]);
    }
}
