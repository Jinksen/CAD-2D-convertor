//! Clicks of the Distance and Radius tools: snap, pick and record the measurement.

use super::{Map, PICK_PX, SNAP_PX};
use crate::state::AppState;
use crate::state::section::camera;
use crate::state::section::measure::{self, Tool};

/// Handles a click at element point `at` for the active tool; false when the tool is Select.
pub(super) fn click(s: &mut AppState, at: [f64; 2]) -> bool {
    let tool = s.section.view.tool;
    if tool == Tool::Select {
        return false;
    }
    let (Some(section), Some(view), Some(size)) =
        (&s.section.last, s.section.camera(), s.section.view.size)
    else {
        return true;
    };
    let map = Map::new(view, size);
    let w = camera::screen_to_view(view, size, at);
    let world = [w[0], -w[1]];
    let hidden = |id: &str| s.model.hidden.contains(id);
    match tool {
        Tool::Distance => {
            let p = measure::snap(section, &hidden, world, SNAP_PX / map.scale);
            s.section.view.measure.click_distance(p);
        }
        _ => {
            let hit = measure::circle_at(section, &hidden, world, PICK_PX / map.scale, map.tol());
            s.section.view.measure.click_radius(hit);
        }
    }
    true
}
