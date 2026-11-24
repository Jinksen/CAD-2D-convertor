//! App-wide state: theme, number format, messages log and backend status.

use std::path::PathBuf;

use rune::TextEdit;
use rune::prefs::Prefs;
use serde::{Deserialize, Serialize};

use crate::api::dto::{Health, Severity};
pub use crate::format::NumberFormat;
use crate::state::layout::Layout;
use crate::state::recent::Recent;
use crate::state::settings::{Category, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::System, Theme::Light, Theme::Dark];

    pub fn is_dark(self, system_dark: bool) -> bool {
        match self {
            Theme::System => system_dark,
            Theme::Light => false,
            Theme::Dark => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum BackendStatus {
    #[default]
    Connecting,
    Online {
        service: String,
        api_version: String,
    },
    Offline(String),
}

impl From<Result<Health, String>> for BackendStatus {
    fn from(result: Result<Health, String>) -> Self {
        match result {
            Ok(h) => BackendStatus::Online {
                service: h.service,
                api_version: h.api_version,
            },
            Err(reason) => BackendStatus::Offline(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub severity: Severity,
    pub text: String,
    /// Body the message is about; clicking it focuses that body.
    pub component_id: Option<String>,
    pub related_component_id: Option<String>,
}

#[derive(Default)]
pub struct ShellState {
    pub theme: Theme,
    pub number_format: NumberFormat,
    pub messages: Vec<Message>,
    pub backend: BackendStatus,
    /// What the service is working on, shown in the status bar while set.
    pub busy: Option<String>,
    pub busy_since: Option<std::time::Instant>,
    /// Bumped by Cancel: a result started under an older value is dropped (like React's abort).
    pub cancel_gen: u64,
    /// The settings dialog is open, on this category.
    pub show_settings: bool,
    pub settings_category: Category,
    pub settings: Settings,
    /// Project text at the last open or save; a difference means unsaved changes.
    pub baseline: Option<String>,
    /// What is waiting for the answer of the unsaved-changes prompt.
    pub pending: Option<crate::session::Pending>,
    /// Recent projects, newest first; kept in the prefs file.
    pub recent: Recent,
    pub recent_open: bool,
    /// A newer autosave exists for the project being opened.
    pub recovery: Option<crate::recovery::Recovery>,
    pub last_autosave: Option<std::time::Instant>,
    /// The window title last handed to the window system.
    pub title: String,
    /// Bound to the default export folder field of the settings dialog.
    pub export_folder: TextEdit,
    /// Where the open project was saved; Ctrl+S saves there.
    pub project_path: Option<PathBuf>,
    /// Bound to the STEP path field of the model settings.
    pub step_path: TextEdit,
    /// A short-lived confirmation shown as a toast: title, detail, when it was raised.
    pub toast: Option<(&'static str, String, std::time::Instant)>,
    /// Panel sizes and collapsed state, remembered on this machine.
    pub layout: Layout,
    /// The messages panel is expanded; opens on a warning or error.
    pub messages_open: bool,
    /// Messages list filter; `None` shows every severity.
    pub severity_filter: Option<Severity>,
    /// Remembers theme and number format on this machine; `None` in tests.
    prefs: Option<Prefs>,
}

impl ShellState {
    /// Loads the remembered theme and number format.
    pub fn with_prefs(prefs: Prefs) -> Self {
        let settings = prefs
            .get::<Settings>("settings")
            .unwrap_or_default()
            .sanitized();
        ShellState {
            recent: Recent::from_strings(prefs.get("recent").unwrap_or_default()),
            export_folder: TextEdit::new(settings.export.folder.clone().unwrap_or_default()),
            settings,
            theme: prefs.get("theme").unwrap_or_default(),
            number_format: prefs.get("number_format").unwrap_or_default(),
            layout: prefs
                .get::<Layout>("layout")
                .unwrap_or_default()
                .sanitized(),
            prefs: Some(prefs),
            ..ShellState::default()
        }
    }

    /// Adds a message; the first warning or error opens the panel.
    pub fn push(&mut self, message: Message) {
        if message.severity != Severity::Info {
            self.messages_open = true;
        }
        self.messages.push(message);
    }

    pub fn log(&mut self, severity: Severity, text: impl Into<String>) {
        self.push(Message {
            severity,
            text: text.into(),
            component_id: None,
            related_component_id: None,
        });
    }

    /// Logs a confirmation and also raises it as a toast (Messages stays the record).
    pub fn notify(&mut self, title: &'static str, text: String) {
        self.log(Severity::Info, text.clone());
        self.toast = Some((title, text, std::time::Instant::now()));
    }

    /// Import, project open and section runs can be cancelled; the others finish in moments.
    pub fn cancellable(&self) -> bool {
        matches!(
            self.busy.as_deref(),
            Some("Importing model" | "Opening project" | "Computing exact section")
        )
    }

    /// Whole seconds the running operation has taken.
    pub fn elapsed_secs(&self) -> u64 {
        self.busy_since.map_or(0, |t| t.elapsed().as_secs())
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.remember();
    }

    pub fn set_format(&mut self, format: NumberFormat) {
        self.number_format = format;
        self.remember();
    }

    /// Applies `change` to the settings and remembers them.
    pub fn edit_settings(&mut self, change: impl FnOnce(&mut Settings)) {
        change(&mut self.settings);
        self.settings = std::mem::take(&mut self.settings).sanitized();
        let settings = self.settings.clone();
        let Some(prefs) = &mut self.prefs else { return };
        if let Err(error) = prefs.set("settings", &settings) {
            self.log(Severity::Warning, error.to_string());
        }
    }

    /// Restores one category to its defaults (theme and number format belong to General and
    /// Units).
    pub fn reset_category(&mut self, category: Category) {
        match category {
            Category::General => {
                self.set_theme(Theme::default());
                self.edit_settings(|s| s.general = Default::default());
            }
            Category::Units => self.set_format(NumberFormat::default()),
            Category::Viewport => self.edit_settings(|s| s.viewport = Default::default()),
            Category::Export => {
                self.edit_settings(|s| s.export = Default::default());
                self.export_folder.replace_text(String::new());
            }
            Category::Service => {}
        }
    }

    /// Puts a project at the top of the recent list and remembers the list.
    pub fn push_recent(&mut self, path: PathBuf) {
        self.recent.push(path);
        self.remember_recent();
    }

    pub fn remove_recent(&mut self, path: &std::path::Path) {
        self.recent.remove(path);
        self.remember_recent();
    }

    fn remember_recent(&mut self) {
        let list = self.recent.to_strings();
        let Some(prefs) = &mut self.prefs else { return };
        if let Err(error) = prefs.set("recent", &list) {
            self.log(Severity::Warning, error.to_string());
        }
    }

    /// Sets (or clears) the default export folder and its field.
    pub fn set_export_folder(&mut self, folder: Option<PathBuf>) {
        let text = folder
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.export_folder.replace_text(text.clone());
        self.edit_settings(|s| s.export.folder = folder.map(|_| text));
    }

    /// Applies the typed folder; it must be an existing folder.
    pub fn commit_export_folder(&mut self) {
        let typed = self.export_folder.text.trim().to_owned();
        if typed.is_empty() {
            return self.set_export_folder(None);
        }
        let path = PathBuf::from(&typed);
        if path.is_dir() {
            self.set_export_folder(Some(path));
        } else {
            self.log(
                Severity::Error,
                format!("\"{typed}\" is not an existing folder."),
            );
        }
    }

    /// Shows the service log folder in the file manager.
    pub fn open_log_folder(&mut self) {
        let dir = crate::backend::log_dir();
        let _ = std::fs::create_dir_all(&dir);
        let program = if cfg!(windows) {
            "explorer"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        if let Err(error) = std::process::Command::new(program).arg(&dir).spawn() {
            self.log(
                Severity::Error,
                format!("Cannot open {}: {error}", dir.display()),
            );
        }
    }

    pub fn remember_layout(&mut self) {
        let layout = self.layout;
        let Some(prefs) = &mut self.prefs else { return };
        if let Err(error) = prefs.set("layout", &layout) {
            self.log(Severity::Warning, error.to_string());
        }
    }

    pub fn count(&self, severity: Severity) -> usize {
        self.messages
            .iter()
            .filter(|m| m.severity == severity)
            .count()
    }

    fn remember(&mut self) {
        let (theme, format) = (self.theme, self.number_format);
        let Some(prefs) = &mut self.prefs else { return };
        let saved = prefs
            .set("theme", &theme)
            .and(prefs.set("number_format", &format));
        if let Err(error) = saved {
            self.log(Severity::Warning, error.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::LengthUnit;

    #[test]
    fn theme_and_format_survive_a_restart() {
        let path = std::env::temp_dir().join(format!("c2m-prefs-{}.json", std::process::id()));
        let mut shell = ShellState::with_prefs(Prefs::at(&path));
        assert_eq!(
            (shell.theme, shell.number_format),
            (Theme::System, NumberFormat::default())
        );
        shell.set_theme(Theme::Dark);
        shell.set_format(NumberFormat {
            length_unit: LengthUnit::In,
            ..NumberFormat::default()
        });
        let again = ShellState::with_prefs(Prefs::at(&path));
        assert_eq!(again.theme, Theme::Dark);
        assert_eq!(again.number_format.length_unit, LengthUnit::In);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn settings_survive_a_restart_and_reset_per_category() {
        let path = std::env::temp_dir().join(format!("c2m-settings-{}.json", std::process::id()));
        let mut shell = ShellState::with_prefs(Prefs::at(&path));
        shell.edit_settings(|s| {
            s.general.recent_count = 5;
            s.viewport.grid_default = true;
            s.export.zip = false;
        });
        shell.set_theme(Theme::Dark);
        let mut again = ShellState::with_prefs(Prefs::at(&path));
        assert_eq!(again.settings.general.recent_count, 5);
        assert!(again.settings.viewport.grid_default && !again.settings.export.zip);
        again.reset_category(Category::Viewport);
        assert!(!again.settings.viewport.grid_default);
        assert_eq!(
            again.settings.general.recent_count, 5,
            "other categories stay"
        );
        again.reset_category(Category::General);
        assert_eq!(
            (again.settings.general.recent_count, again.theme),
            (10, Theme::System)
        );
        let third = ShellState::with_prefs(Prefs::at(&path));
        assert_eq!(third.settings.general.recent_count, 10);
        assert!(!third.settings.export.zip, "export kept");
        std::fs::remove_file(path).unwrap();
    }
}
