//! App-wide state: theme, number format, messages log and backend status.

use crate::api::dto::{Health, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn is_dark(self, system_dark: bool) -> bool {
        match self {
            Theme::System => system_dark,
            Theme::Light => false,
            Theme::Dark => true,
        }
    }
}

/// Display notation only; geometry and export stay in mm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberFormat {
    pub length_decimals: u8,
}

impl Default for NumberFormat {
    fn default() -> Self {
        NumberFormat { length_decimals: 3 }
    }
}

impl NumberFormat {
    pub fn length(&self, mm: f64) -> String {
        format!("{mm:.*}", usize::from(self.length_decimals))
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
}

impl ShellState {
    pub fn log(&mut self, severity: Severity, text: impl Into<String>) {
        self.messages.push(Message {
            severity,
            text: text.into(),
            component_id: None,
            related_component_id: None,
        });
    }
}
