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

pub fn write_project(path: &Path, contents: &str) -> Result<(), String> {
    check_path(path)?;
    if contents.len() > MAX_PROJECT_BYTES {
        return Err("The project file exceeds the 16 MiB limit.".into());
    }
    fs::write(path, contents).map_err(|error| format!("Cannot write the project file: {error}"))
}

pub fn read_project(path: &Path) -> Result<String, String> {
    check_path(path)?;
    let meta =
        fs::metadata(path).map_err(|error| format!("Cannot read the project file: {error}"))?;
    if meta.len() > MAX_PROJECT_BYTES as u64 {
        return Err("The project file exceeds the 16 MiB limit.".into());
    }
    fs::read_to_string(path).map_err(|error| format!("Cannot read the project file: {error}"))
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
