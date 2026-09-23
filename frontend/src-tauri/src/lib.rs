pub const APPLICATION_IDENTIFIER: &str = "com.cad2maxwell.desktop";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
