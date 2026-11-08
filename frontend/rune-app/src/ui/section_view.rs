//! 2D section drawing. Owned by the 2D viewport work; `viewport` calls `canvas` in 2D mode.

use rune::tok::Theme;
use rune::{El, col};

use crate::state::AppState;

pub fn canvas(_s: &AppState, _t: &Theme) -> El {
    col().key("section-canvas").w_full().grow(1.0)
}
