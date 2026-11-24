use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;

pub fn check_archive(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_ARCHIVE_BYTES || !bytes.starts_with(b"PK\x03\x04") {
        return Err("The DXF bundle is invalid or exceeds the 64 MiB save limit.".into());
    }
    Ok(())
}

pub fn write_archive(path: &Path, bytes: &[u8]) -> Result<(), String> {
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

/// Unpacks the bundle into the new folder `dir` (never into an existing one), so the result
/// is the same files without the ZIP. Entry names that would leave the folder are refused.
pub fn extract_archive(dir: &Path, bytes: &[u8]) -> Result<(), String> {
    check_archive(bytes)?;
    if !dir.is_absolute() {
        return Err("Choose an absolute folder path for the export.".into());
    }
    if dir.exists() {
        return Err("That folder already exists. Choose a new name.".into());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|error| format!("The DXF bundle cannot be read: {error}"))?;
    let write = |zip: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>| -> Result<(), String> {
        fs::create_dir_all(dir).map_err(|error| format!("Cannot create the folder: {error}"))?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|error| error.to_string())?;
            let name = entry
                .enclosed_name()
                .filter(|_| !entry.is_dir())
                .ok_or("The DXF bundle contains an unsafe file name.")?;
            let target = dir.join(name);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut file = fs::File::create(&target)
                .map_err(|error| format!("Cannot write {}: {error}", target.display()))?;
            std::io::copy(&mut entry, &mut file).map_err(|error| error.to_string())?;
        }
        Ok(())
    };
    write(&mut zip).inspect_err(|_| {
        let _ = fs::remove_dir_all(dir);
    })
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

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut out = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut out);
        for (name, body) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
        out.into_inner()
    }

    #[test]
    fn extracting_writes_the_files_into_a_new_folder_only() {
        let dir = directory();
        let bytes = zip_of(&[("a.dxf", b"0\nEOF"), ("manifest.json", b"{}")]);
        let target = dir.join("out");
        super::extract_archive(&target, &bytes).unwrap();
        assert_eq!(fs::read(target.join("a.dxf")).unwrap(), b"0\nEOF");
        assert!(super::extract_archive(&target, &bytes).is_err(), "exists");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn extracting_refuses_entries_that_escape_the_folder_and_leaves_nothing() {
        let dir = directory();
        let bytes = zip_of(&[("ok.dxf", b"1"), ("../evil.txt", b"x")]);
        let target = dir.join("out");
        assert!(super::extract_archive(&target, &bytes).is_err());
        assert!(!target.exists() && !dir.join("evil.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
