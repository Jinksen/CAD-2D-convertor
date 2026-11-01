use std::fs;
use std::path::Path;

const MAX_PROJECT_BYTES: usize = 16 * 1024 * 1024;

fn check_path(path: &Path) -> Result<(), String> {
    if path.is_absolute()
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("c2mproj"))
    {
        Ok(())
    } else {
        Err("Choose an absolute path ending in .c2mproj.".into())
    }
}

fn write_project(path: &Path, contents: &str) -> Result<(), String> {
    check_path(path)?;
    if contents.len() > MAX_PROJECT_BYTES {
        return Err("The project file exceeds the 16 MiB limit.".into());
    }
    fs::write(path, contents).map_err(|error| format!("Cannot write the project file: {error}"))
}

fn read_project(path: &Path) -> Result<String, String> {
    check_path(path)?;
    let meta =
        fs::metadata(path).map_err(|error| format!("Cannot read the project file: {error}"))?;
    if meta.len() > MAX_PROJECT_BYTES as u64 {
        return Err("The project file exceeds the 16 MiB limit.".into());
    }
    fs::read_to_string(path).map_err(|error| format!("Cannot read the project file: {error}"))
}

/// Saves to `path`, or asks for one when it is `None`. Returns the saved path; `None` = cancelled.
#[tauri::command]
pub async fn save_project(
    contents: String,
    path: Option<String>,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut target = match path {
            Some(path) => path.into(),
            None => {
                let Some(picked) = rfd::FileDialog::new()
                    .set_title("Save CAD2Maxwell project")
                    .add_filter("CAD2Maxwell project", &["c2mproj"])
                    .set_file_name("project.c2mproj")
                    .save_file()
                else {
                    return Ok(None);
                };
                picked
            }
        };
        if target.extension().is_none() {
            target.set_extension("c2mproj");
        }
        write_project(&target, &contents)?;
        Ok(Some(target.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|error| format!("The save dialog could not complete: {error}"))?
}

/// Asks for a project file; returns `(path, contents)`, or `None` when cancelled.
#[tauri::command]
pub async fn open_project() -> Result<Option<(String, String)>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Open CAD2Maxwell project")
            .add_filter("CAD2Maxwell project", &["c2mproj"])
            .pick_file()
        else {
            return Ok(None);
        };
        let contents = read_project(&path)?;
        Ok(Some((path.to_string_lossy().into_owned(), contents)))
    })
    .await
    .map_err(|error| format!("The open dialog could not complete: {error}"))?
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn round_trips_and_rejects_wrong_paths() {
        let dir = std::env::temp_dir().join(format!("cad2maxwell-proj-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.c2mproj");
        super::write_project(&path, "{\"x\":1}").unwrap();
        super::write_project(&path, "{\"x\":2}").unwrap();
        assert_eq!(super::read_project(&path).unwrap(), "{\"x\":2}");
        assert!(super::write_project(&dir.join("a.json"), "{}").is_err());
        assert!(super::read_project(Path::new("relative.c2mproj")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
