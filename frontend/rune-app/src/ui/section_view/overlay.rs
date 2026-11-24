//! Drawing aids of the 2D view: grid, origin axes, scale bar, cursor readout, hover name and
//! the legend of visible groups. Display only; none of it reaches the section data or export.

use rune::tok::{Theme, space, type_};
use rune::{El, LineCap, Shape, col, el, row, text};

use super::{Look, Map, P, rgb, visible_box};
use crate::format::NumberFormat;
use crate::state::AppState;
use crate::state::section::camera::{self, Size};
use crate::state::section::measure::{Measure, Reading, Tool};

const GRID_TARGET_PX: f64 = 60.0;
const MAX_GRID_LINES: usize = 120;
const BAR_TARGET_PX: f64 = 100.0;
const MARGIN: f32 = 12.0;
const LEGEND_ROWS: usize = 8;

/// Grid and origin axes, painted under the geometry.
pub(super) fn under(mut shape: Shape, map: &Map, size: Size, look: &Look, grid: bool) -> Shape {
    let shown = visible_box(map, size, 0.0);
    if grid {
        let step = camera::nice_step(GRID_TARGET_PX / map.scale);
        let line = look.faint;
        for x in camera::grid_positions(shown[0], shown[2], step, MAX_GRID_LINES) {
            shape = shape.line(
                map.px([x, shown[1]]),
                map.px([x, shown[3]]),
                1.0,
                LineCap::Butt,
                line,
            );
        }
        for y in camera::grid_positions(shown[1], shown[3], step, MAX_GRID_LINES) {
            shape = shape.line(
                map.px([shown[0], y]),
                map.px([shown[2], y]),
                1.0,
                LineCap::Butt,
                line,
            );
        }
    }
    if shown[0] <= 0.0 && shown[2] >= 0.0 {
        shape = shape.line(
            map.px([0.0, shown[1]]),
            map.px([0.0, shown[3]]),
            1.0,
            LineCap::Butt,
            look.axis_y,
        );
    }
    if shown[1] <= 0.0 && shown[3] >= 0.0 {
        shape = shape.line(
            map.px([shown[0], 0.0]),
            map.px([shown[2], 0.0]),
            1.0,
            LineCap::Butt,
            look.axis_x,
        );
    }
    shape
}

/// The scale bar's pixel length and its label, in the display unit.
pub(super) fn bar(map: &Map, size: Size, fmt: &NumberFormat) -> (f32, String) {
    let k = fmt.length_unit.mm_per_unit();
    let (units, px) = camera::scale_bar(map.view, size, k, BAR_TARGET_PX);
    (px as f32, fmt.length(units * k))
}

/// Scale bar at the bottom right, painted over the geometry.
pub(super) fn over(shape: Shape, map: &Map, size: Size, look: &Look, fmt: &NumberFormat) -> Shape {
    let (px, _) = bar(map, size, fmt);
    let (w, h) = (size.width as f32, size.height as f32);
    let (x1, y) = (w - MARGIN, h - 26.0);
    let x0 = x1 - px;
    shape
        .line([x0, y], [x1, y], 2.0, LineCap::Butt, look.ink)
        .line([x0, y - 4.0], [x0, y + 4.0], 1.0, LineCap::Butt, look.ink)
        .line([x1, y - 4.0], [x1, y + 4.0], 1.0, LineCap::Butt, look.ink)
}

/// Cursor coordinates in the display unit, plus the export name of the region under it.
pub(super) fn readout(s: &AppState) -> Option<String> {
    let [x, y] = s.section.view.cursor?;
    let f = &s.shell.number_format;
    let name = s
        .model
        .hover
        .as_deref()
        .and_then(|id| {
            s.model
                .imported
                .as_ref()?
                .components
                .iter()
                .find(|c| c.id == id)
        })
        .map(|c| format!("  |  {}", c.export_name))
        .unwrap_or_default();
    Some(format!("x {}  y {}{name}", f.length(x), f.length(y)))
}

/// Visible groups with the color of their first colored member.
pub(super) fn legend(s: &AppState) -> Vec<(String, Option<[u8; 3]>)> {
    let Some(imp) = &s.model.imported else {
        return vec![];
    };
    s.model
        .groups
        .iter()
        .filter(|g| !s.model.group_hidden(&g.name))
        .map(|g| {
            let color = imp
                .components
                .iter()
                .filter(|c| g.component_ids.contains(&c.id))
                .find_map(|c| c.source_color);
            (g.name.clone(), color)
        })
        .take(LEGEND_ROWS)
        .collect()
}

/// Text overlays, pinned to the corners of the canvas; they never take pointer input.
pub(super) fn labels(s: &AppState, t: &Theme, map: Option<&Map>, size: Option<Size>) -> Vec<El> {
    let small = |msg: String| text(msg).font(type_::LABEL).color(t.text_secondary);
    let mut out = vec![];
    if let Some(msg) = measure_text(s) {
        out.push(
            small(msg)
                .key("measure-reading")
                .role("status")
                .pin(Some(MARGIN), Some(6.0), None, None)
                .inert(),
        );
    }
    if let Some(msg) = readout(s) {
        out.push(
            small(msg)
                .key("section-readout")
                .pin(Some(MARGIN), None, None, Some(6.0))
                .inert(),
        );
    }
    if let (Some(map), Some(size)) = (map, size) {
        out.push(
            small(bar(map, size, &s.shell.number_format).1)
                .key("section-scale")
                .pin(None, None, Some(MARGIN), Some(6.0))
                .inert(),
        );
    }
    let entries = legend(s);
    if !entries.is_empty() {
        let mut c = col().key("section-legend").gap(space::S1).inert().pin(
            None,
            Some(6.0),
            Some(MARGIN),
            None,
        );
        for (name, color) in entries {
            let swatch = el()
                .w(10.0)
                .h(10.0)
                .fill(color.map_or(t.text_secondary, rgb));
            c = c.child(
                row()
                    .gap(space::S1)
                    .items_center()
                    .child(swatch)
                    .child(small(name)),
            );
        }
        out.push(c);
    }
    out
}

/// Markers of the current measurement, painted over the geometry.
pub(super) fn marks(mut shape: Shape, map: &Map, look: &Look, m: &Measure) -> Shape {
    let cross = |shape: Shape, p: P| {
        let [x, y] = map.px(p);
        shape
            .line(
                [x - 5.0, y],
                [x + 5.0, y],
                1.5,
                LineCap::Butt,
                look.selected,
            )
            .line(
                [x, y - 5.0],
                [x, y + 5.0],
                1.5,
                LineCap::Butt,
                look.selected,
            )
    };
    if let Some(p) = m.first {
        shape = cross(shape, p);
    }
    match &m.reading {
        Some(Reading::Distance { from, to }) => {
            shape = cross(cross(shape, *from), *to);
            shape.line(
                map.px(*from),
                map.px(*to),
                1.5,
                LineCap::Round,
                look.selected,
            )
        }
        Some(Reading::Radius { center, radius }) => {
            let c = cross(shape, *center);
            c.line(
                map.px(*center),
                map.px([center[0] + radius, center[1]]),
                1.5,
                LineCap::Round,
                look.selected,
            )
        }
        None => shape,
    }
}

/// What the active tool wants next, or the reading it produced.
pub(super) fn measure_text(s: &AppState) -> Option<String> {
    let v = &s.section.view;
    let f = &s.shell.number_format;
    let at = |p: P| {
        format!(
            "({}{}{})",
            f.length(p[0]),
            f.list_separator(),
            f.length(p[1])
        )
    };
    match (v.tool, &v.measure.reading) {
        (Tool::Select, _) => None,
        (_, Some(Reading::Distance { from, to })) => {
            let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
            Some(format!(
                "Distance {}  (dx {}, dy {})",
                f.length(dx.hypot(dy)),
                f.length(dx),
                f.length(dy)
            ))
        }
        (_, Some(Reading::Radius { center, radius })) => Some(format!(
            "Radius {}  centre {}",
            f.length(*radius),
            at(*center)
        )),
        (Tool::Distance, None) if v.measure.first.is_some() => {
            Some("Distance: click the second point.".into())
        }
        (Tool::Distance, None) => {
            Some("Distance: click two points. They snap to curve ends and arc centres.".into())
        }
        (Tool::Radius, None) => Some(
            v.measure
                .note
                .clone()
                .unwrap_or_else(|| "Radius: click an arc or circle.".into()),
        ),
    }
}
