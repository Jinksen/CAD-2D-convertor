//! `.c2mproj` files: settings and a reference to the STEP file, never geometry.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::dto::Severity;
use crate::format::NumberFormat;
use crate::state::AppState;
use crate::state::model::BodyGroup;
use crate::state::section::PlaneKind;

const MAX_PROJECT_BYTES: usize = 16 * 1024 * 1024;
pub const PROJECT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepRef {
    pub path: String,
    pub sha256: String,
    /// The STEP path relative to the project's folder, when the STEP lies inside it, so a
    /// project folder can be moved or shared as a whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneRef {
    pub kind: String,
    pub normal_xyz: [f64; 3],
    pub offset_mm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub app: String,
    pub version: u32,
    pub step: StepRef,
    pub plane: PlaneRef,
    #[serde(default)]
    pub number_format: NumberFormat,
    /// Absent in files saved before body visibility existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_component_ids: Option<Vec<String>>,
    /// Optional fields added in 0.3; older versions of the app ignore them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<BodyGroup>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collapsed_folders: Vec<String>,
}

/// `step` relative to `dir` with forward slashes, if `step` lies inside `dir`.
pub fn relative_to(step: &Path, dir: &Path) -> Option<String> {
    let rest = step.strip_prefix(dir).ok()?;
    let parts: Vec<String> = rest
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

impl StepRef {
    /// The saved absolute path, else the relative one resolved against the project's folder.
    pub fn resolve(&self, project: &Path) -> PathBuf {
        let absolute = PathBuf::from(&self.path);
        if absolute.is_file() {
            return absolute;
        }
        self.relative_path
            .as_ref()
            .zip(project.parent())
            .map(|(relative, dir)| dir.join(relative))
            .filter(|path| path.is_file())
            .unwrap_or(absolute)
    }
}

pub fn kind_name(kind: PlaneKind) -> &'static str {
    match kind {
        PlaneKind::XY => "XY",
        PlaneKind::XZ => "XZ",
        PlaneKind::YZ => "YZ",
        PlaneKind::Custom => "custom",
    }
}

pub fn parse_kind(name: &str) -> Option<PlaneKind> {
    [
        PlaneKind::XY,
        PlaneKind::XZ,
        PlaneKind::YZ,
        PlaneKind::Custom,
    ]
    .into_iter()
    .find(|k| kind_name(*k) == name)
}

/// Parses project text; the error is user-readable.
pub fn parse_project(text: &str) -> Result<ProjectFile, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|_| "The project file is not valid JSON.".to_string())?;
    if value["app"] == "CAD2Maxwell"
        && let Some(version) = value["version"].as_u64()
        && version > u64::from(PROJECT_VERSION)
    {
        return Err(format!(
            "The project file version {version} is newer than this application supports ({PROJECT_VERSION}). Update CAD2Maxwell to open it."
        ));
    }
    serde_json::from_value::<ProjectFile>(value)
        .ok()
        .filter(|p| {
            p.app == "CAD2Maxwell"
                && p.version == PROJECT_VERSION
                && parse_kind(&p.plane.kind).is_some()
        })
        .ok_or_else(|| "The file is not a valid CAD2Maxwell project.".to_string())
}

pub fn serialize_project(project: &ProjectFile) -> String {
    let mut text = serde_json::to_string_pretty(project).unwrap_or_default();
    text.push('\n');
    text
}

impl AppState {
    /// The current session as a project; `None` before a STEP file is imported.
    pub fn project_file(&self) -> Option<ProjectFile> {
        let imported = self.model.imported.as_ref()?;
        Some(ProjectFile {
            app: "CAD2Maxwell".into(),
            version: PROJECT_VERSION,
            step: StepRef {
                path: imported.path.clone(),
                sha256: imported.sha256.clone(),
                relative_path: None,
            },
            plane: PlaneRef {
                kind: kind_name(self.section.plane).into(),
                normal_xyz: self.section.custom_normal,
                offset_mm: self.section.offset_mm,
            },
            number_format: self.shell.number_format,
            hidden_component_ids: Some(self.model.hidden.iter().cloned().collect()),
            groups: self.model.groups.clone(),
            collapsed_folders: self.model.collapsed.iter().cloned().collect(),
        })
    }

    /// Writes the project to `path` (chosen with a save dialog) and remembers it for Ctrl+S.
    pub fn save_project(&mut self, mut path: PathBuf) {
        let Some(mut project) = self.project_file() else {
            self.shell
                .log(Severity::Error, "Open a STEP file before saving a project.");
            return;
        };
        if path.extension().is_none() {
            path.set_extension("c2mproj");
        }
        project.step.relative_path = path
            .parent()
            .and_then(|dir| relative_to(Path::new(&project.step.path), dir));
        match write_project(&path, &serialize_project(&project)) {
            Ok(()) => {
                let text = format!("Project saved to {}", path.display());
                self.shell.notify("Project saved", text);
                self.shell.project_path = Some(path);
            }
            Err(error) => self.shell.log(Severity::Error, error),
        }
    }
}

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
    // Write beside the target, then rename over it, so a crash never leaves a half-written project.
    let temp = path.with_extension("c2mproj.tmp");
    fs::write(&temp, contents)
        .and_then(|()| fs::rename(&temp, path))
        .map_err(|error| {
            let _ = fs::remove_file(&temp);
            format!("Cannot write the project file: {error}")
        })
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

    use std::sync::{Mutex, mpsc};
    use std::time::Duration;

    use super::*;
    use crate::api::Client;
    use crate::api::fake::serve;
    use crate::state::model::tests::summary;
    use crate::tasks::{Tasks, Update};

    fn import_json(sha: &str) -> Vec<u8> {
        let component = |id: &str| {
            format!(
                r#"{{"id":"{id}","source_id":null,"source_name":null,"display_name":"{id}","export_name":"{id}_1",
                "hierarchy_path":[],"source_color":null,"name_provenance":null,"color_provenance":null,"body_ordinal":0,
                "bounds_mm":{{"min_xyz":[0,0,0],"max_xyz":[10,20,30]}}}}"#
            )
        };
        format!(
            r#"{{"import_id":"i2","path":"m.step","sha256":"{sha}","source_unit":"mm","to_mm_scale":1.0,
            "bounds_mm":{{"min_xyz":[0,0,0],"max_xyz":[10,20,30]}},"component_count":2,
            "components":[{},{}],"diagnostics":[]}}"#,
            component("b1"),
            component("b2")
        )
        .into_bytes()
    }

    /// Saves a session with an XZ plane at 4.5 mm and b2 hidden, then reopens it against a
    /// service that reports `sha` for the STEP file.
    fn reopen(sha: &str) -> AppState {
        let dir = std::env::temp_dir().join(format!("c2m-proj-open-{}-{sha}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut saved = AppState::default();
        saved.model.set_imported(summary(&["b1", "b2"]));
        saved.section.plane = PlaneKind::XZ;
        saved.section.offset_mm = Some(4.5);
        saved.model.hidden = ["b2".to_string(), "gone".to_string()].into();
        saved.model.groups = vec![BodyGroup {
            name: "Magnets".into(),
            component_ids: vec!["b1".into(), "gone".into()],
        }];
        saved.model.collapsed = ["Motor".to_string()].into();
        saved.save_project(dir.join("p"));
        let path = saved.shell.project_path.clone().expect("saved path");
        assert_eq!(path.extension().unwrap(), "c2mproj");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("\"components\"") && text.contains("\"version\": 1"));

        let fake = serve(vec![(200, "application/json", import_json(sha))]);
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let tasks = Tasks::new(Client::new(&fake.url, None), move |u: Update| {
            tx.lock().unwrap().send(u).unwrap()
        });
        let mut state = AppState {
            tasks: Some(tasks),
            ..AppState::default()
        };
        state.open_project(path.clone());
        rx.recv_timeout(Duration::from_secs(5))
            .expect("import result")(&mut state);
        assert_eq!(state.shell.project_path.as_ref(), Some(&path));
        std::fs::remove_dir_all(dir).unwrap();
        state
    }

    #[test]
    fn round_trips_plane_hidden_bodies_and_drops_ids_the_step_no_longer_has() {
        let state = reopen("x");
        assert_eq!(
            (state.section.plane, state.section.offset_mm),
            (PlaneKind::XZ, Some(4.5))
        );
        assert_eq!(state.model.hidden, ["b2".to_string()].into());
        assert_eq!(state.model.groups[0].component_ids, ["b1"]);
        assert!(state.model.collapsed.contains("Motor"));
        let last = state.shell.messages.last().unwrap();
        assert_eq!(
            (last.severity, last.text.as_str()),
            (Severity::Info, "Project opened.")
        );
    }

    #[test]
    fn warns_visibly_when_the_step_hash_changed() {
        let state = reopen("changed");
        let last = state.shell.messages.last().unwrap();
        assert_eq!(last.severity, Severity::Warning);
        assert!(last.text.contains("SHA-256 mismatch"));
    }

    #[test]
    fn rejects_future_versions_and_malformed_files() {
        let mut state = AppState::default();
        state.model.set_imported(summary(&["b1"]));
        let text = serialize_project(&state.project_file().unwrap());
        assert!(parse_project(&text).is_ok());
        let newer = text.replace("\"version\": 1", "\"version\": 2");
        assert!(parse_project(&newer).unwrap_err().contains("newer"));
        assert!(parse_project("{}").unwrap_err().contains("not a valid"));
        assert!(parse_project("nope").unwrap_err().contains("JSON"));
        let bad_plane = text.replace("\"XY\"", "\"AB\"");
        assert!(parse_project(&bad_plane).is_err());
    }

    #[test]
    fn saving_without_a_model_is_an_error() {
        let mut state = AppState::default();
        state.save_project(PathBuf::from("C:/p.c2mproj"));
        assert_eq!(
            state.shell.messages.last().unwrap().severity,
            Severity::Error
        );
    }

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
        assert!(!dir.join("a.c2mproj.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_moved_project_folder_finds_its_step_by_the_relative_path() {
        let dir = std::env::temp_dir().join(format!("c2m-proj-rel-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("cad")).unwrap();
        let step = dir.join("cad").join("m.step");
        std::fs::write(&step, "x").unwrap();
        assert_eq!(relative_to(&step, &dir).as_deref(), Some("cad/m.step"));
        assert_eq!(relative_to(Path::new("D:/other/m.step"), &dir), None);
        let moved = StepRef {
            path: "Z:/old/place/cad/m.step".into(),
            sha256: "x".into(),
            relative_path: Some("cad/m.step".into()),
        };
        assert_eq!(moved.resolve(&dir.join("p.c2mproj")), step);
        let lost = StepRef {
            relative_path: None,
            ..moved
        };
        assert_eq!(
            lost.resolve(&dir.join("p.c2mproj")),
            PathBuf::from("Z:/old/place/cad/m.step")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
