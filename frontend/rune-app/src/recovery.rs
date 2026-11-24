//! Autosave: `<project>.c2mproj.autosave` is written every few minutes while the session has
//! unsaved changes, offered when the project is next opened if it is newer, and removed by a
//! successful save.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::api::dto::Severity;
use crate::project::{read_project, serialize_project};
use crate::state::AppState;

pub const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(180);

pub fn autosave_path(project: &Path) -> PathBuf {
    let mut name = project.as_os_str().to_owned();
    name.push(".autosave");
    PathBuf::from(name)
}

/// The autosave of `project`, when it exists and is newer than the project file.
pub fn newer_autosave(project: &Path) -> Option<PathBuf> {
    let auto = autosave_path(project);
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    (modified(&auto)? > modified(project)?).then_some(auto)
}

/// Offered when a project with a newer autosave is opened.
#[derive(Debug, Clone, PartialEq)]
pub struct Recovery {
    pub project: PathBuf,
    pub autosave: PathBuf,
}

impl AppState {
    /// Opens a `.c2mproj`; a newer autosave is offered first.
    pub fn open_project(&mut self, path: PathBuf) {
        let text = match read_project(&path) {
            Ok(text) => text,
            Err(error) => return self.shell.log(Severity::Error, error),
        };
        match newer_autosave(&path) {
            Some(autosave) => {
                self.shell.recovery = Some(Recovery {
                    project: path,
                    autosave,
                })
            }
            None => self.open_project_text(path, text, false),
        }
    }

    /// Answer to the recovery prompt: open the autosave or the saved project.
    pub fn resolve_recovery(&mut self, restore: bool) {
        let Some(Recovery { project, autosave }) = self.shell.recovery.take() else {
            return;
        };
        let source = if restore { &autosave } else { &project };
        match std::fs::read_to_string(source) {
            Ok(text) => self.open_project_text(project, text, restore),
            Err(error) => {
                let text = format!("Cannot read {}: {error}", source.display());
                self.shell.log(Severity::Error, text)
            }
        }
    }

    /// Writes the autosave when there are unsaved changes and a project path to put it beside.
    pub fn autosave_now(&mut self) -> bool {
        let (Some(path), true) = (self.shell.project_path.clone(), self.is_dirty()) else {
            return false;
        };
        let Some(project) = self.project_file() else {
            return false;
        };
        let target = autosave_path(&path);
        let temp = target.with_extension("autosave.tmp");
        let written = std::fs::write(&temp, serialize_project(&project))
            .and_then(|()| std::fs::rename(&temp, &target));
        if let Err(error) = written {
            let _ = std::fs::remove_file(&temp);
            self.shell
                .log(Severity::Warning, format!("Autosave failed: {error}"));
            return false;
        }
        true
    }

    /// Autosaves when the interval has passed since the last one.
    pub fn autosave_due(&mut self, now: Instant) -> bool {
        let due = self
            .shell
            .last_autosave
            .is_none_or(|t| now.duration_since(t) >= AUTOSAVE_INTERVAL);
        if due && self.autosave_now() {
            self.shell.last_autosave = Some(now);
            return true;
        }
        false
    }

    pub fn remove_autosave(&self, project: &Path) {
        let _ = std::fs::remove_file(autosave_path(project));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::model::tests::summary;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("c2m-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn dirty_session(project: PathBuf) -> AppState {
        let mut s = AppState::default();
        s.model.set_imported(summary(&["a"]));
        s.mark_clean();
        s.shell.project_path = Some(project);
        s.model.toggle_hidden("a");
        s
    }

    #[test]
    fn autosave_is_written_only_when_dirty_with_a_path_and_a_save_removes_it() {
        let d = dir("auto");
        let project = d.join("p.c2mproj");
        let mut s = dirty_session(project.clone());
        assert!(s.autosave_now());
        assert_eq!(autosave_path(&project), d.join("p.c2mproj.autosave"));
        assert!(autosave_path(&project).is_file());
        s.save_project(project.clone());
        assert!(!autosave_path(&project).exists(), "removed after a save");
        assert!(!s.autosave_now(), "clean sessions write nothing");
        let mut no_path = dirty_session(project);
        no_path.shell.project_path = None;
        assert!(!no_path.autosave_now());
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn the_interval_gates_autosaves() {
        let d = dir("interval");
        let mut s = dirty_session(d.join("p.c2mproj"));
        let t0 = Instant::now();
        assert!(s.autosave_due(t0));
        assert!(!s.autosave_due(t0 + Duration::from_secs(10)));
        assert!(s.autosave_due(t0 + AUTOSAVE_INTERVAL));
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn only_a_newer_autosave_is_offered_and_the_choice_picks_the_source() {
        let d = dir("offer");
        let project = d.join("p.c2mproj");
        std::fs::write(&project, "{}").unwrap();
        assert_eq!(newer_autosave(&project), None);
        let auto = autosave_path(&project);
        std::fs::write(&auto, "{}").unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(60);
        std::fs::File::options()
            .write(true)
            .open(&auto)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert_eq!(newer_autosave(&project), None, "older autosave is ignored");
        std::fs::write(&auto, "{}").unwrap();
        assert_eq!(newer_autosave(&project), Some(auto.clone()));
        let mut s = AppState::default();
        s.open_project(project.clone());
        assert_eq!(s.shell.recovery.as_ref().map(|r| &r.autosave), Some(&auto));
        s.resolve_recovery(false);
        assert!(s.shell.recovery.is_none());
        std::fs::remove_dir_all(d).unwrap();
    }
}
