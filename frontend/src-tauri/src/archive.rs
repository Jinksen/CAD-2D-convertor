use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;

fn check_archive(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_ARCHIVE_BYTES || !bytes.starts_with(b"PK\x03\x04") {
        return Err("The DXF bundle is invalid or exceeds the 64 MiB save limit.".into());
    }
    Ok(())
}

fn write_archive(path: &Path, bytes: &[u8]) -> Result<(), String> {
    check_archive(bytes)?;
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        return Err("Choose an absolute path ending in .zip for the DXF bundle.".into());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::AlreadyExists => {
                "That file already exists. Choose a new name.".into()
            }
            _ => format!("Cannot create the DXF bundle: {error}"),
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(format!("Cannot write the DXF bundle: {error}"));
    }
    Ok(())
}

#[tauri::command]
pub async fn save_section_archive(bytes: Vec<u8>) -> Result<Option<String>, String> {
    check_archive(&bytes)?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(mut path) = rfd::FileDialog::new()
            .set_title("Save CAD2Maxwell DXF bundle")
            .add_filter("DXF bundle (ZIP)", &["zip"])
            .set_file_name("section.zip")
            .save_file()
        else {
            return Ok(None);
        };
        if path.extension().is_none() {
            path.set_extension("zip");
        }
        write_archive(&path, &bytes)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|error| format!("The save dialog could not complete: {error}"))?
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cad2maxwell-save-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn saves_archive_bytes_and_never_overwrites_existing_files() {
        let dir = directory();
        let path = dir.join("section.zip");
        let bytes = b"PK\x03\x04test archive";
        super::write_archive(&path, bytes).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(super::write_archive(&path, b"PK\x03\x04replacement").is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn refuses_non_archive_content_and_wrong_extension_without_writing() {
        let dir = directory();
        let zip = dir.join("section.zip");
        let dxf = dir.join("section.dxf");
        assert!(super::write_archive(&zip, b"not a zip").is_err());
        assert!(super::write_archive(&dxf, b"PK\x03\x04archive").is_err());
        assert!(!zip.exists());
        assert!(!dxf.exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
