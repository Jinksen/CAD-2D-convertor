//! Application state, composed of per-feature sub-states so features can evolve independently.
//! Workflows that touch several sub-states or the service live in `tasks`.

pub mod model;
pub mod preview;
pub mod section;
pub mod shell;

use std::sync::OnceLock;
use std::time::Instant;

use crate::tasks::Tasks;

#[derive(Default)]
pub struct AppState {
    pub shell: shell::ShellState,
    pub model: model::ModelState,
    pub section: section::SectionState,
    pub preview: preview::PreviewState,
    /// Set by `main`; `None` in headless layout tests, where service actions only log.
    pub tasks: Option<Tasks>,
}

/// Monotonic milliseconds for undo-history coalescing.
pub fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    u64::try_from(START.get_or_init(Instant::now).elapsed().as_millis()).unwrap_or(u64::MAX)
}
