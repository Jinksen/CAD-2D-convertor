//! 2D camera math in SVG-like view-box terms (y down, `ViewBox.y` is the top edge), ported from
//! `viewCamera.ts`. Section coordinates are y up, so the world point (x, y) sits at view y = -y.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// A y-up bounding box (`y` is the bottom edge) as a view box.
pub fn from_world(b: ViewBox) -> ViewBox {
    ViewBox {
        y: -b.y - b.height,
        ..b
    }
}

pub fn fit_view(bounds: ViewBox) -> ViewBox {
    let margin = bounds.width.max(bounds.height).max(1.0) * 0.08;
    ViewBox {
        x: bounds.x - margin,
        y: bounds.y - margin,
        width: bounds.width + margin * 2.0,
        height: bounds.height + margin * 2.0,
    }
}

pub fn zoom_view(view: ViewBox, factor: f64, anchor: Option<[f64; 2]>) -> ViewBox {
    let [ax, ay] = anchor.unwrap_or([view.x + view.width / 2.0, view.y + view.height / 2.0]);
    ViewBox {
        x: ax + (view.x - ax) * factor,
        y: ay + (view.y - ay) * factor,
        width: view.width * factor,
        height: view.height * factor,
    }
}

/// Screen pixels per view unit, meet-fitted (preserveAspectRatio xMidYMid meet).
pub fn scale(view: ViewBox, size: Size) -> f64 {
    (size.width / view.width).min(size.height / view.height)
}

fn letterbox(view: ViewBox, size: Size) -> (f64, [f64; 2]) {
    let s = scale(view, size);
    (
        s,
        [
            (size.width - view.width * s) / 2.0,
            (size.height - view.height * s) / 2.0,
        ],
    )
}

/// Screen point (pixels in the element) to view-box units.
pub fn screen_to_view(view: ViewBox, size: Size, p: [f64; 2]) -> [f64; 2] {
    let (s, o) = letterbox(view, size);
    [view.x + (p[0] - o[0]) / s, view.y + (p[1] - o[1]) / s]
}

/// Section (y up) point to screen pixels.
#[cfg(test)]
pub fn world_to_screen(view: ViewBox, size: Size, p: [f64; 2]) -> [f64; 2] {
    let (s, o) = letterbox(view, size);
    [(p[0] - view.x) * s + o[0], (-p[1] - view.y) * s + o[1]]
}

pub fn units_per_pixel(view: ViewBox, size: Size) -> f64 {
    (view.width / size.width).max(view.height / size.height)
}

pub fn pan_view(view: ViewBox, dx: f64, dy: f64) -> ViewBox {
    ViewBox {
        x: view.x + dx,
        y: view.y + dy,
        ..view
    }
}

/// The 1-2-5 step (in `unit` multiples) nearest below `target`, for scale bars and grids.
pub fn nice_step(target: f64) -> f64 {
    if !(target.is_finite() && target > 0.0) {
        return 1.0;
    }
    let base = 10f64.powf(target.log10().floor());
    [5.0, 2.0, 1.0]
        .into_iter()
        .map(|m| m * base)
        .find(|&v| v <= target)
        .unwrap_or(base)
}

/// Grid positions (multiples of `step`) inside `[lo, hi]`; empty when there would be too many.
pub fn grid_positions(lo: f64, hi: f64, step: f64, max: usize) -> Vec<f64> {
    if !(step > 0.0 && lo.is_finite() && hi.is_finite()) || (hi - lo) / step > max as f64 {
        return vec![];
    }
    let first = (lo / step).ceil() as i64;
    let last = (hi / step).floor() as i64;
    (first..=last).map(|i| i as f64 * step).collect()
}

/// Scale bar of about `target_px`: its length in `mm_per_unit` display units and its pixels.
pub fn scale_bar(view: ViewBox, size: Size, mm_per_unit: f64, target_px: f64) -> (f64, f64) {
    let px_per_mm = scale(view, size);
    let units = nice_step(target_px / px_per_mm / mm_per_unit);
    (units, units * mm_per_unit * px_per_mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: ViewBox = ViewBox {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 50.0,
    };

    #[test]
    fn thin_sections_fit_into_a_finite_padded_view() {
        let fitted = fit_view(ViewBox {
            x: 5.0,
            y: 2.0,
            width: 0.0,
            height: 0.0,
        });
        assert!(fitted.width > 0.0 && fitted.height > 0.0);
        assert!((fitted.x + fitted.width / 2.0 - 5.0).abs() < 1e-12);
        let wide = fit_view(VIEW);
        assert_eq!((wide.x, wide.width), (-8.0, 116.0));
    }

    #[test]
    fn zoom_keeps_the_anchor_fixed() {
        let size = Size {
            width: 400.0,
            height: 400.0,
        };
        let cursor = [300.0, 120.0];
        let anchor = screen_to_view(VIEW, size, cursor);
        let zoomed = zoom_view(VIEW, 0.5, Some(anchor));
        assert_eq!((zoomed.width, zoomed.height), (50.0, 25.0));
        let after = screen_to_view(zoomed, size, cursor);
        assert!((after[0] - anchor[0]).abs() < 1e-9 && (after[1] - anchor[1]).abs() < 1e-9);
    }

    #[test]
    fn screen_mapping_letterboxes_and_flips_y() {
        let size = Size {
            width: 200.0,
            height: 200.0,
        };
        // 100x50 into 200x200: scale 2, 50 px of vertical bars each side.
        assert_eq!(screen_to_view(VIEW, size, [0.0, 50.0]), [0.0, 0.0]);
        assert_eq!(screen_to_view(VIEW, size, [200.0, 150.0]), [100.0, 50.0]);
        let p = world_to_screen(VIEW, size, [10.0, -5.0]);
        assert_eq!(p, [20.0, 60.0]);
        assert_eq!(units_per_pixel(VIEW, size), 0.5);
    }

    #[test]
    fn pan_moves_the_camera_without_resizing() {
        let moved = pan_view(VIEW, 3.0, -2.0);
        assert_eq!((moved.x, moved.y, moved.width), (3.0, -2.0, 100.0));
    }

    #[test]
    fn world_bounds_flip_to_view_coordinates() {
        let v = from_world(ViewBox {
            x: 0.0,
            y: 2.0,
            width: 10.0,
            height: 3.0,
        });
        assert_eq!((v.y, v.height), (-5.0, 3.0));
    }

    #[test]
    fn nice_steps_follow_one_two_five() {
        for (t, want) in [
            (0.7, 0.5),
            (1.0, 1.0),
            (3.0, 2.0),
            (7.5, 5.0),
            (130.0, 100.0),
        ] {
            assert_eq!(nice_step(t), want, "{t}");
        }
        assert_eq!(nice_step(f64::NAN), 1.0);
    }

    #[test]
    fn grid_lines_stay_inside_the_view_and_are_capped() {
        assert_eq!(grid_positions(-1.0, 4.5, 2.0, 100), vec![0.0, 2.0, 4.0]);
        assert!(grid_positions(0.0, 1e6, 1.0, 100).is_empty());
    }

    #[test]
    fn scale_bar_length_is_a_round_number_near_the_target() {
        let size = Size {
            width: 200.0,
            height: 200.0,
        };
        // 2 px per mm: 100 px target is 50 mm.
        let (units, px) = scale_bar(VIEW, size, 1.0, 100.0);
        assert_eq!((units, px), (50.0, 100.0));
        // In inches the bar is a round number of inches, converted back to pixels.
        let (inch, px) = scale_bar(VIEW, size, 25.4, 100.0);
        assert_eq!(inch, 1.0);
        assert!((px - 50.8).abs() < 1e-9);
    }
}
