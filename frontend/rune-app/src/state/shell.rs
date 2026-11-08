//! App-wide state: theme, number format, messages log and backend status.

use std::path::PathBuf;

use rune::TextEdit;
use rune::prefs::Prefs;
use serde::{Deserialize, Serialize};

use crate::api::dto::{Health, Severity};
pub use crate::format::NumberFormat;

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
    /// The preferences panel replaces the settings column while open.
    pub show_prefs: bool,
    /// Where the open project was saved; Ctrl+S saves there.
    pub project_path: Option<PathBuf>,
    /// Bound to the STEP path field of the model settings.
    pub step_path: TextEdit,
    /// Remembers theme and number format on this machine; `None` in tests.
    prefs: Option<Prefs>,
}

impl ShellState {
    /// Loads the remembered theme and number format.
    pub fn with_prefs(prefs: Prefs) -> Self {
        ShellState {
            theme: prefs.get("theme").unwrap_or_default(),
            number_format: prefs.get("number_format").unwrap_or_default(),
            prefs: Some(prefs),
            ..ShellState::default()
        }
    }

    pub fn log(&mut self, severity: Severity, text: impl Into<String>) {
        self.messages.push(Message {
            severity,
            text: text.into(),
            component_id: None,
            related_component_id: None,
        });
    }

    /// Import, project open and section runs can be cancelled; the others finish in moments.
    pub fn cancellable(&self) -> bool {
        matches!(
            self.busy.as_deref(),
            Some("Importing STEP" | "Opening project" | "Computing exact section")
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
}
