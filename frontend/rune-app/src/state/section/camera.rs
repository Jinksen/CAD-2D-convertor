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
}
