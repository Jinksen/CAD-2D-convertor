//! Panel sizes and collapsed state, remembered between runs in the Rune prefs file.

use serde::{Deserialize, Serialize};

pub const TREE_DEFAULT: f32 = 260.0;
pub const PROPS_DEFAULT: f32 = 320.0;
pub const MESSAGES_DEFAULT: f32 = 150.0;
pub const TREE_RANGE: (f32, f32) = (160.0, 520.0);
pub const PROPS_RANGE: (f32, f32) = (240.0, 600.0);
pub const MESSAGES_RANGE: (f32, f32) = (80.0, 520.0);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub tree_width: f32,
    pub props_width: f32,
    pub messages_height: f32,
    pub tree_collapsed: bool,
    pub props_collapsed: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            tree_width: TREE_DEFAULT,
            props_width: PROPS_DEFAULT,
            messages_height: MESSAGES_DEFAULT,
            tree_collapsed: false,
            props_collapsed: false,
        }
    }
}

fn clamp(v: f32, (lo, hi): (f32, f32)) -> f32 {
    if v.is_finite() { v.clamp(lo, hi) } else { lo }
}

impl Layout {
    /// A saved layout with sizes forced back into range (a hand-edited prefs file).
    pub fn sanitized(self) -> Layout {
        Layout {
            tree_width: clamp(self.tree_width, TREE_RANGE),
            props_width: clamp(self.props_width, PROPS_RANGE),
            messages_height: clamp(self.messages_height, MESSAGES_RANGE),
            ..self
        }
    }

    #[allow(dead_code)] // wired to the panel splitters in the next change
    pub fn set_tree_width(&mut self, w: f32) {
        self.tree_width = clamp(w, TREE_RANGE);
    }

    #[allow(dead_code)]
    pub fn set_props_width(&mut self, w: f32) {
        self.props_width = clamp(w, PROPS_RANGE);
    }

    pub fn set_messages_height(&mut self, h: f32) {
        self.messages_height = clamp(h, MESSAGES_RANGE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_clamped_and_bad_saved_values_are_repaired() {
        let mut l = Layout::default();
        l.set_tree_width(10.0);
        l.set_props_width(9000.0);
        l.set_messages_height(f32::NAN);
        assert_eq!(
            (l.tree_width, l.props_width, l.messages_height),
            (TREE_RANGE.0, PROPS_RANGE.1, MESSAGES_RANGE.0)
        );
        let bad: Layout = serde_json::from_str(r#"{"tree_width": 1.0}"#).unwrap();
        assert_eq!(bad.props_width, PROPS_DEFAULT);
        assert_eq!(bad.sanitized().tree_width, TREE_RANGE.0);
    }
}
