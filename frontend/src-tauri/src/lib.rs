pub const APPLICATION_IDENTIFIER: &str = "com.cad2maxwell.desktop";
mod archive;
mod backend;
mod project;

use tauri::{Manager, RunEvent};

#[tauri::command]
fn backend_session_token(backend: tauri::State<'_, backend::Backend>) -> Option<String> {
    backend.token.clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let backend = backend::start(app.handle());
            app.manage(backend);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend_session_token,
            archive::save_section_archive,
            project::save_project,
            project::open_project
        ])
        .build(tauri::generate_context!())
        .expect("failed to build CAD2Maxwell desktop host")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                backend::stop(&app.state::<backend::Backend>());
            }
        });
}

#[cfg(test)]
mod tests {
    use super::APPLICATION_IDENTIFIER;

    #[test]
    fn application_identity_is_stable() {
        assert_eq!(APPLICATION_IDENTIFIER, "com.cad2maxwell.desktop");
    }
}
