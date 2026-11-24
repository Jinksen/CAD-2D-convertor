//! Holes for Rune's simple-polygon fill: each hole is joined to its outer boundary by a
//! zero-width bridge, giving one weakly simple polygon whose interior excludes the hole.
//! Display tessellation only; the section data and the export are never touched.

use super::Drawn;
use crate::api::dto::WireRole;
use crate::state::section::geometry::inside;

type P = [f64; 2];
/// Ear clipping is quadratic; longer polygons are stroked but not filled.
const MAX_FILL_POINTS: usize = 3000;

/// Each outer ring of a body with the holes inside it (smallest enclosing outer wins).
pub fn rings(wires: &[Drawn]) -> Vec<(Vec<P>, Vec<Vec<P>>)> {
    let ring = |w: &Drawn| -> Vec<P> {
        w.lines[0]
            .iter()
            .map(|p| [f64::from(p[0]), f64::from(p[1])])
            .collect()
    };
    let closed = |role: WireRole| {
        wires
            .iter()
            .filter(move |w| {
                w.closed && w.role == Some(role) && w.lines[0].len() <= MAX_FILL_POINTS
            })
            .map(ring)
    };
    let mut out: Vec<(Vec<P>, Vec<Vec<P>>)> =
        closed(WireRole::Outer).map(|o| (o, vec![])).collect();
    for h in closed(WireRole::Hole) {
        let owner = out
            .iter_mut()
            .filter(|(o, _)| inside(o, h[0]))
            .min_by(|a, b| area(&a.0).abs().total_cmp(&area(&b.0).abs()));
        if let Some((_, holes)) = owner {
            holes.push(h);
        }
    }
    out
}

/// Shoelace area, positive for counter-clockwise.
pub fn area(poly: &[P]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

fn cross(o: P, a: P, b: P) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

/// Segments `ab` and `cd` cross at an interior point of both.
fn crosses(a: P, b: P, c: P, d: P) -> bool {
    let (d1, d2, d3, d4) = (
        cross(a, b, c),
        cross(a, b, d),
        cross(c, d, a),
        cross(c, d, b),
    );
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// One polygon covering `outer` minus `holes` . Returns
/// None when a hole cannot be bridged; the caller then fills nothing rather than a wrong shape.
pub fn with_holes(outer: &[P], holes: &[&[P]]) -> Option<Vec<P>> {
    let ccw = |r: &[P], want: bool| -> Vec<P> {
        let mut v = r.to_vec();
        if (area(&v) > 0.0) != want {
            v.reverse();
        }
        v
    };
    let mut poly = ccw(outer, true);
    let mut rings: Vec<Vec<P>> = holes.iter().map(|h| ccw(h, false)).collect();
    // Rightmost holes first, so a bridge never has to cross a hole still to come.
    let right = |r: &Vec<P>| r.iter().map(|p| p[0]).fold(f64::MIN, f64::max);
    rings.sort_by(|a, b| right(b).total_cmp(&right(a)));
    let edges = |rs: &[Vec<P>]| -> Vec<(P, P)> {
        rs.iter()
            .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
            .collect()
    };
    let obstacles = edges(&rings);
    let (outer_ring, mut bridges) = (poly.clone(), Vec::<(P, P)>::new());
    for ring in &rings {
        let k = (0..ring.len()).max_by(|&a, &b| ring[a][0].total_cmp(&ring[b][0]))?;
        let m = ring[k];
        let mut cand: Vec<usize> = (0..poly.len()).collect();
        let d2 = |p: P| (p[0] - m[0]).powi(2) + (p[1] - m[1]).powi(2);
        cand.sort_by(|&a, &b| d2(poly[a]).total_cmp(&d2(poly[b])));
        let clear = |v: P| {
            let mid = [(v[0] + m[0]) / 2.0, (v[1] + m[1]) / 2.0];
            inside(&outer_ring, mid)
                && !rings.iter().any(|h| inside(h, mid))
                && !obstacles
                    .iter()
                    .chain(edges(std::slice::from_ref(&outer_ring)).iter())
                    .chain(bridges.iter())
                    .any(|&(c, d)| crosses(v, m, c, d))
        };
        let at = cand.into_iter().find(|&i| clear(poly[i]))?;
        let v = poly[at];
        bridges.push((v, m));
        let mut walk: Vec<P> = (0..=ring.len())
            .map(|j| ring[(k + j) % ring.len()])
            .collect();
        walk.push(v);
        poly.splice(at + 1..at + 1, walk);
    }
    Some(poly)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<P> {
        vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
    }

    #[test]
    fn bridged_polygon_has_the_area_of_the_outer_minus_the_holes() {
        let outer = rect(0.0, 0.0, 10.0, 10.0);
        let (a, b) = (rect(2.0, 2.0, 4.0, 4.0), rect(6.0, 5.0, 8.0, 9.0));
        let poly = with_holes(&outer, &[&a, &b]).expect("bridged");
        assert!((area(&poly).abs() - (100.0 - 4.0 - 8.0)).abs() < 1e-9);
        assert_eq!(poly.len(), 4 + 2 * (4 + 2));
    }

    #[test]
    fn no_holes_keeps_the_outer_ring() {
        let outer = rect(0.0, 0.0, 3.0, 3.0);
        assert_eq!(with_holes(&outer, &[]).unwrap(), outer);
    }
}
