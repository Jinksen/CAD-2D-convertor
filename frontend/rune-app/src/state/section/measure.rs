//! 2D measuring: distance between two (snapped) points, and radius and centre of an arc or
//! circle. Works on the analytic section curves; display only, nothing is written back.

use crate::api::dto::{SectionCurve, SectionResponse};

use super::geometry;

type P = [f64; 2];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Select,
    Distance,
    Radius,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reading {
    Distance { from: P, to: P },
    Radius { center: P, radius: f64 },
}

#[derive(Debug, Default)]
pub struct Measure {
    /// First point of a distance, waiting for the second.
    pub first: Option<P>,
    pub reading: Option<Reading>,
    /// Why the last click measured nothing.
    pub note: Option<String>,
}

/// Curve endpoints and arc centres, the points a measurement snaps to.
fn snap_points(c: &SectionCurve) -> Vec<P> {
    let mut v = vec![geometry::start_point(c), geometry::end_point(c)];
    match c {
        SectionCurve::Circle { center, .. } | SectionCurve::Ellipse { center, .. } => {
            v.push(*center)
        }
        SectionCurve::Line { .. } => {}
    }
    v
}

/// The snap point nearest to `at` within `reach`, else `at` itself.
pub fn snap(section: &SectionResponse, hidden: &dyn Fn(&str) -> bool, at: P, reach: f64) -> P {
    let near = [at[0] - reach, at[1] - reach, at[0] + reach, at[1] + reach];
    let mut best: Option<(f64, P)> = None;
    for comp in section
        .components
        .iter()
        .filter(|c| !hidden(&c.component_id))
    {
        for c in comp.wires.iter().flat_map(|w| &w.curves) {
            let b = geometry::curve_box(c);
            if b[0] > near[2] || b[2] < near[0] || b[1] > near[3] || b[3] < near[1] {
                continue;
            }
            for p in snap_points(c) {
                let d = (p[0] - at[0]).hypot(p[1] - at[1]);
                if d <= reach && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, p));
                }
            }
        }
    }
    best.map_or(at, |(_, p)| p)
}

/// Centre and radius of the circle or arc nearest to `at` within `reach` (`tol` tessellates).
pub fn circle_at(
    section: &SectionResponse,
    hidden: &dyn Fn(&str) -> bool,
    at: P,
    reach: f64,
    tol: f64,
) -> Option<(P, f64)> {
    let mut best: Option<(f64, P, f64)> = None;
    for comp in section
        .components
        .iter()
        .filter(|c| !hidden(&c.component_id))
    {
        for c in comp.wires.iter().flat_map(|w| &w.curves) {
            let SectionCurve::Circle { center, radius, .. } = c else {
                continue;
            };
            let b = geometry::curve_box(c);
            if b[0] - reach > at[0]
                || b[2] + reach < at[0]
                || b[1] - reach > at[1]
                || b[3] + reach < at[1]
            {
                continue;
            }
            let mut pts = vec![];
            geometry::tessellate(c, tol, false, &mut pts);
            let d = pts
                .windows(2)
                .map(|s| geometry::segment_distance(s[0], s[1], at))
                .fold(f64::MAX, f64::min);
            if d <= reach && best.is_none_or(|(bd, ..)| d < bd) {
                best = Some((d, *center, *radius));
            }
        }
    }
    best.map(|(_, c, r)| (c, r))
}

impl Measure {
    pub fn clear(&mut self) {
        *self = Measure::default();
    }

    /// A click in Distance mode at the (already snapped) point `p`.
    pub fn click_distance(&mut self, p: P) {
        self.note = None;
        match self.first.take() {
            Some(from) => self.reading = Some(Reading::Distance { from, to: p }),
            None => {
                self.first = Some(p);
                self.reading = None;
            }
        }
    }

    /// A click in Radius mode; `hit` is the circle found under the pointer.
    pub fn click_radius(&mut self, hit: Option<(P, f64)>) {
        match hit {
            Some((center, radius)) => {
                self.reading = Some(Reading::Radius { center, radius });
                self.note = None;
            }
            None => {
                self.reading = None;
                self.note = Some("No arc or circle under the pointer.".into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::dto::{SectionComponent, SectionPlane, SectionWire, WireRole};

    fn section() -> SectionResponse {
        let line = |a: P, b: P| SectionCurve::Line { start: a, end: b };
        SectionResponse {
            section_id: "s".into(),
            import_id: "i".into(),
            plane: SectionPlane::XY { offset_mm: 0.0 },
            components: vec![SectionComponent {
                component_id: "a".into(),
                wires: vec![
                    SectionWire {
                        closed: true,
                        role: Some(WireRole::Outer),
                        curves: vec![
                            line([0.0, 0.0], [10.0, 0.0]),
                            line([10.0, 0.0], [10.0, 10.0]),
                            line([10.0, 10.0], [0.0, 10.0]),
                            line([0.0, 10.0], [0.0, 0.0]),
                        ],
                    },
                    SectionWire {
                        closed: true,
                        role: Some(WireRole::Hole),
                        curves: vec![SectionCurve::Circle {
                            center: [5.0, 5.0],
                            radius: 2.0,
                            x_axis: [1.0, 0.0],
                            y_axis: [0.0, 1.0],
                            start_parameter: 0.0,
                            end_parameter: std::f64::consts::TAU,
                        }],
                    },
                ],
            }],
            diagnostics: vec![],
        }
    }

    #[test]
    fn points_snap_to_endpoints_and_arc_centres_only_within_reach() {
        let s = section();
        let shown = |_: &str| false;
        assert_eq!(snap(&s, &shown, [10.2, 0.1], 0.5), [10.0, 0.0]);
        assert_eq!(snap(&s, &shown, [5.1, 4.9], 0.5), [5.0, 5.0], "arc centre");
        assert_eq!(
            snap(&s, &shown, [3.0, 0.0], 0.5),
            [3.0, 0.0],
            "mid-edge stays"
        );
        let hidden = |id: &str| id == "a";
        assert_eq!(snap(&s, &hidden, [10.2, 0.1], 0.5), [10.2, 0.1]);
    }

    #[test]
    fn two_clicks_measure_a_distance_and_the_third_starts_over() {
        let mut m = Measure::default();
        m.click_distance([0.0, 0.0]);
        assert!(m.reading.is_none() && m.first.is_some());
        m.click_distance([3.0, 4.0]);
        assert_eq!(
            m.reading,
            Some(Reading::Distance {
                from: [0.0, 0.0],
                to: [3.0, 4.0]
            })
        );
        m.click_distance([1.0, 1.0]);
        assert!(m.reading.is_none() && m.first == Some([1.0, 1.0]));
    }

    #[test]
    fn a_click_on_a_circle_reads_its_radius_and_centre() {
        let s = section();
        let shown = |_: &str| false;
        let mut m = Measure::default();
        m.click_radius(circle_at(&s, &shown, [7.0, 5.05], 0.3, 0.01));
        assert_eq!(
            m.reading,
            Some(Reading::Radius {
                center: [5.0, 5.0],
                radius: 2.0
            })
        );
        m.click_radius(circle_at(&s, &shown, [1.0, 1.0], 0.3, 0.01));
        assert!(m.reading.is_none() && m.note.is_some());
    }
}
