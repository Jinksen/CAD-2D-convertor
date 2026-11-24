//! Drag handles between panels. Rune has no splitter widget, so this follows the table's
//! column resize handle: `on_drag` with the start size kept in `Drag::data`.

use rune::tok::Theme;
use rune::{Cursor, Drag, El, Key, Phase, el, row};

use crate::state::AppState;

/// A draggable divider. `vertical` is a bar between side-by-side panels (drag left/right);
/// otherwise a bar between stacked ones (drag up/down). `sign` is -1 when growing the panel
/// means dragging the other way (the panel to the right of, or below, the bar).
pub struct Splitter {
    pub key: &'static str,
    pub label: &'static str,
    pub vertical: bool,
    pub sign: f32,
    pub get: fn(&AppState) -> f32,
    pub set: fn(&mut AppState, f32),
    pub reset: fn(&mut AppState),
}

const THICKNESS: f32 = 5.0;
const KEY_STEP: f32 = 16.0;

impl Splitter {
    pub fn build(self, t: &Theme) -> El {
        let Splitter {
            key,
            label,
            vertical,
            sign,
            get,
            set,
            reset,
        } = self;
        let line = if vertical {
            el().w(1.0).h_full().ml(2.0)
        } else {
            el().h(1.0).w_full().mt(2.0)
        }
        .fill(t.separator)
        .a11y_hidden();
        let bar = if vertical {
            row().w(THICKNESS).h_full().cursor(Cursor::ResizeColumn)
        } else {
            row().h(THICKNESS).w_full().cursor(Cursor::ResizeRow)
        };
        bar.key(key)
            .role("separator")
            .label(label)
            .focusable(true)
            .hover(t.hover)
            .on_drag(move |s: &mut AppState, d: &mut Drag, _| {
                match d.phase {
                    Phase::Down => d.data = get(s),
                    _ if d.cancelled() => set(s, d.data),
                    _ => {
                        let delta = if vertical { d.dx } else { d.dy };
                        set(s, d.data + sign * delta);
                    }
                }
                if d.phase == Phase::Up {
                    s.shell.remember_layout();
                }
            })
            .on_double_press(move |s: &mut AppState| {
                reset(s);
                s.shell.remember_layout();
            })
            .on_key(move |s: &mut AppState, ev, _| {
                let grow = match (&ev.key, vertical) {
                    (Key::Right, true) | (Key::Down, false) => sign * KEY_STEP,
                    (Key::Left, true) | (Key::Up, false) => -sign * KEY_STEP,
                    _ => return false,
                };
                set(s, get(s) + grow);
                s.shell.remember_layout();
                true
            })
            .child(line)
    }
}
