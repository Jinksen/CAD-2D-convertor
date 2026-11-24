//! The session as a document: the unsaved-changes marker, the prompt before something replaces
//! the session, recent projects and the window title.
//!
//! A change is any difference between the project text now and at the last open or save (the
//! baseline), so undoing a change makes the session clean again.

use std::path::PathBuf;

use rune::Tx;

use crate::api::dto::Severity;
use crate::project::serialize_project;
use crate::state::AppState;

/// What waits behind the unsaved-changes prompt.
#[derive(Debug, Clone, PartialEq)]
pub enum Pending {
    OpenModel(PathBuf),
    OpenProject(PathBuf),
    Quit,
}

impl AppState {
    fn project_text(&self) -> Option<String> {
        self.project_file().map(|p| serialize_project(&p))
    }

    /// Takes the current project as the saved state.
    pub fn mark_clean(&mut self) {
        self.shell.baseline = self.project_text();
    }

    pub fn is_dirty(&self) -> bool {
        match (&self.shell.baseline, self.project_text()) {
            (Some(base), Some(now)) => *base != now,
            _ => false,
        }
    }

    /// Window title: `* name - CAD2Maxwell` while there are unsaved changes.
    pub fn window_title(&self) -> String {
        let name = self
            .shell
            .project_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|n| n.to_string_lossy().into_owned())
            .or_else(|| {
                let path = std::path::Path::new(&self.model.imported.as_ref()?.path);
                Some(path.file_name()?.to_string_lossy().into_owned())
            });
        match name {
            Some(name) => format!(
                "{}{name} - CAD2Maxwell",
                if self.is_dirty() { "*" } else { "" }
            ),
            None => "CAD2Maxwell".into(),
        }
    }

    /// Runs `what` now, or asks first when it would discard unsaved changes.
    pub fn guard(&mut self, what: Pending, tx: &mut Tx) {
        if self.is_dirty() && self.shell.settings.general.confirm_unsaved {
            self.shell.pending = Some(what);
        } else {
            self.run(what, tx);
        }
    }

    pub fn run(&mut self, what: Pending, tx: &mut Tx) {
        match what {
            Pending::OpenModel(path) => self.open_step(path),
            Pending::OpenProject(path) => self.open_project(path),
            Pending::Quit => tx.quit(),
        }
    }

    /// Prompt answer "Discard".
    pub fn discard_and_continue(&mut self, tx: &mut Tx) {
        if let Some(what) = self.shell.pending.take() {
            self.run(what, tx);
        }
    }

    /// Prompt answer "Save": saves to the project path (or asks for one), then continues.
    pub fn save_and_continue(&mut self, tx: &mut Tx) {
        let Some(what) = self.shell.pending.clone() else {
            return;
        };
        if let Some(path) = self.shell.project_path.clone() {
            self.save_project(path);
            if !self.is_dirty() {
                self.shell.pending = None;
                self.run(what, tx);
            }
            return;
        }
        let options = rune::FileDialogOptions {
            title: "Save project".into(),
            file_name: Some("project.c2mproj".into()),
            ..Default::default()
        }
        .filter("CAD2Maxwell project", &["c2mproj"]);
        tx.save_file(options, move |s: &mut AppState, result, tx| match result {
            Ok(Some(path)) => {
                s.save_project(path);
                if !s.is_dirty() {
                    s.shell.pending = None;
                    s.run(what, tx);
                }
            }
            Ok(None) => {}
            Err(error) => s.shell.log(Severity::Error, error.to_string()),
        });
    }

    /// Opens a recent project, or says why it cannot be opened.
    pub fn open_recent(&mut self, path: PathBuf, tx: &mut Tx) {
        if path.is_file() {
            self.guard(Pending::OpenProject(path), tx);
        } else {
            let text = format!(
                "{} no longer exists. Remove it from the list.",
                path.display()
            );
            self.shell.log(Severity::Warning, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::state::AppState;
    use crate::state::model::tests::summary;

    fn session() -> AppState {
        let mut s = AppState::default();
        s.model.set_imported(summary(&["a"]));
        s.mark_clean();
        s
    }

    #[test]
    fn changes_mark_the_session_and_undoing_them_cleans_it() {
        let mut s = session();
        assert!(!s.is_dirty() && !s.window_title().starts_with('*'));
        s.model.toggle_hidden("a");
        assert!(s.is_dirty());
        assert!(s.window_title().starts_with('*'));
        s.model.toggle_hidden("a");
        assert!(!s.is_dirty());
    }

    #[test]
    fn a_session_without_a_model_is_never_dirty() {
        let s = AppState::default();
        assert!(!s.is_dirty());
        assert_eq!(s.window_title(), "CAD2Maxwell");
    }
}
