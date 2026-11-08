//! CAD2Maxwell native desktop app (Rune). Geometry stays in the local Python/OpenCascade service.

mod api;
mod archive;
mod backend;
// Project open/save is wired in migration phase 3.
#[allow(dead_code)]
mod project;
mod state;
mod tasks;
mod ui;

use rune::{App, AppSpec};

use crate::api::Client;
use crate::state::AppState;
use crate::tasks::Tasks;

/// Service start-up polls for about ten seconds before reporting it offline.
const HEALTH_ATTEMPTS: u32 = 40;

fn main() {
    let backend = backend::start();
    let client = Client::from_env(backend.token.clone());
    let spec = AppSpec {
        min_size: Some((960.0, 600.0)),
        ..AppSpec::new("CAD2Maxwell", (1440.0, 900.0), AppState::default, ui::view)
    };
    let app = App::new(spec);
    // `AppHandle::update` runs the closure on the UI thread and wakes the event loop.
    let handle = app.handle();
    let tasks = Tasks::new(client, move |update| {
        handle.update(move |state, _| update(state))
    });
    app.init(move || {
        let mut state = AppState {
            tasks: Some(tasks),
            ..AppState::default()
        };
        state.check_service(HEALTH_ATTEMPTS);
        state
    })
    .run();
    backend::stop(&backend);
}
