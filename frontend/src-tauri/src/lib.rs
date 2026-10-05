pub const APPLICATION_IDENTIFIER: &str = "com.cad2maxwell.desktop";
mod archive;

#[tauri::command]
fn backend_session_token() -> Option<String> {
    std::env::var("CAD2MAXWELL_SESSION_TOKEN").ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            backend_session_token,
            archive::save_section_archive
        ])
        .run(tauri::generate_context!())
        .expect("failed to run CAD2Maxwell desktop host");
}

#[cfg(test)]
mod tests {
    use super::APPLICATION_IDENTIFIER;

    #[test]
    fn application_identity_is_stable() {
        assert_eq!(APPLICATION_IDENTIFIER, "com.cad2maxwell.desktop");
    }
}
