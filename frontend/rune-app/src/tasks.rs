//! Service calls on worker threads, and the workflows (import, section, export) that use them.
//!
//! A worker never touches state: it posts a closure through `Tasks::post`, which `main` wires
//! to `rune::AppHandle::update` (runs on the UI thread and wakes the event loop).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::api::Client;
use crate::api::dto::{Health, ImportSummary, SectionResponse, Severity};
use crate::archive;
use crate::state::AppState;
use crate::state::preview::ViewportMode;
use crate::state::section::{PlaneKind, PlaneSnapshot, SectionStats};
use crate::state::shell::{BackendStatus, Message};

pub type Update = Box<dyn FnOnce(&mut AppState) + Send>;

#[derive(Clone)]
pub struct Tasks {
    client: Arc<Client>,
    post: Arc<dyn Fn(Update) + Send + Sync>,
}

impl Tasks {
    pub fn new(client: Client, post: impl Fn(Update) + Send + Sync + 'static) -> Self {
        Tasks {
            client: Arc::new(client),
            post: Arc::new(post),
        }
    }

    /// Runs `work` on a new thread, then `done` on the UI thread with its result.
    pub fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Client) -> Result<T, String> + Send + 'static,
        done: impl FnOnce(&mut AppState, Result<T, String>) + Send + 'static,
    ) {
        let tasks = self.clone();
        std::thread::spawn(move || {
            let result = work(&tasks.client);
            (tasks.post)(Box::new(move |state| done(state, result)));
        });
    }
}

fn is_step(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("step") || e.eq_ignore_ascii_case("stp"))
}

impl AppState {
    /// Starts a service call labelled `what` unless one is already running.
    fn start<T: Send + 'static>(
        &mut self,
        what: &str,
        work: impl FnOnce(&Client) -> Result<T, String> + Send + 'static,
        done: impl FnOnce(&mut AppState, T) + Send + 'static,
    ) {
        let Some(tasks) = self.tasks.clone() else {
            self.shell.log(
                Severity::Error,
                "The geometry service client is not available.",
            );
            return;
        };
        if let Some(busy) = &self.shell.busy {
            let text = format!("Wait for the current operation to finish ({busy}).");
            self.shell.log(Severity::Warning, text);
            return;
        }
        self.shell.busy = Some(what.to_owned());
        self.shell.busy_since = Some(std::time::Instant::now());
        let generation = self.shell.cancel_gen;
        tasks.run(work, move |state, result| {
            if state.shell.cancel_gen != generation {
                return; // cancelled: the worker's result is abandoned
            }
            state.shell.busy = None;
            state.shell.busy_since = None;
            match result {
                Ok(value) => done(state, value),
                Err(message) => state.shell.log(Severity::Error, message),
            }
        });
    }

    /// Abandons the running operation; its result is dropped when it arrives. Like React, a
    /// cancelled section also clears the shown result.
    pub fn cancel_operation(&mut self) {
        if !self.shell.cancellable() {
            return;
        }
        if self.shell.busy.as_deref() == Some("Computing exact section") {
            self.section.last = None;
            self.section.stats = None;
        }
        self.shell.busy = None;
        self.shell.busy_since = None;
        self.shell.cancel_gen += 1;
        self.shell.log(Severity::Info, "Cancelled.");
    }

    /// Polls the service health, retrying while it starts up.
    pub fn check_service(&mut self, attempts: u32) {
        let Some(tasks) = self.tasks.clone() else {
            return;
        };
        self.shell.backend = BackendStatus::Connecting;
        tasks.run(
            move |client| {
                let mut result: Result<Health, String> = Err(String::new());
                for attempt in 0..attempts.max(1) {
                    result = client.health();
                    if result.is_ok() {
                        break;
                    }
                    if attempt + 1 < attempts {
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }
                Ok(result)
            },
            |state, result| state.shell.backend = result.and_then(|r| r).into(),
        );
    }

    pub fn open_step(&mut self, path: PathBuf) {
        if !is_step(&path) {
            self.shell
                .log(Severity::Error, "Choose a STEP or STP file.");
            return;
        }
        let service_path = path.to_string_lossy().into_owned();
        self.start(
            "Importing STEP",
            move |client| client.import_step(&service_path),
            move |state, summary| state.finish_import(summary),
        );
    }

    fn finish_import(&mut self, summary: ImportSummary) {
        let text = format!(
            "Imported {} bodies from {}.",
            summary.component_count, summary.path
        );
        self.shell.log(Severity::Info, text);
        for d in &summary.diagnostics {
            self.shell.messages.push(Message {
                severity: d.severity,
                text: d.message.clone(),
                component_id: d.component_id.clone(),
                related_component_id: None,
            });
        }
        self.section.restore(PlaneSnapshot {
            kind: PlaneKind::XY,
            offset_mm: None,
            custom_normal: [0.0, 0.0, 1.0],
        });
        self.section.stats = None;
        self.model.set_imported(summary);
        self.ensure_preview();
    }

    pub fn compute_section(&mut self) {
        // Typed plane fields that were not submitted yet count as the plane to cut.
        self.section.commit_fields(crate::state::now_ms());
        let Some(imported) = &self.model.imported else {
            self.shell.log(
                Severity::Warning,
                "Open a STEP file before computing a section.",
            );
            return;
        };
        let Some(plane) = self.section.request(&imported.bounds_mm) else {
            self.shell.log(
                Severity::Error,
                "The custom plane normal must be a non-zero vector.",
            );
            return;
        };
        let import_id = imported.import_id.clone();
        let sent = plane.clone();
        self.start(
            "Computing exact section",
            move |client| client.section(&import_id, &plane),
            move |state, section| state.finish_section(section, &sent),
        );
    }

    fn finish_section(&mut self, section: SectionResponse, sent: &crate::api::dto::SectionPlane) {
        // A response for an earlier import or another plane kind is stale; drop it.
        if self.model.import_id() != Some(section.import_id.as_str())
            || !self.section.plane_matches(&section.plane)
        {
            return;
        }
        let stats = SectionStats::of(&section);
        let text = format!(
            "Section {sent:?}: {} closed outer regions, {} errors, {} warnings.",
            stats.regions, stats.errors, stats.warnings
        );
        self.shell.log(
            if stats.errors > 0 {
                Severity::Error
            } else {
                Severity::Info
            },
            text,
        );
        for d in &section.diagnostics {
            self.shell.messages.push(Message {
                severity: d.severity,
                text: d.message.clone(),
                component_id: d.component_id.clone(),
                related_component_id: d.related_component_id.clone(),
            });
        }
        // The plane may have been edited while the cut ran: show the result, but as stale.
        let current = self
            .model
            .imported
            .as_ref()
            .and_then(|i| self.section.request(&i.bounds_mm));
        self.section.stats = (current.as_ref() == Some(sent)).then_some(stats);
        self.section.last = Some(section);
    }

    /// Downloads the DXF bundle of the last section into `path` (chosen with a save dialog).
    pub fn export_section(&mut self, mut path: PathBuf) {
        if let Some(reason) = self.section.export_blocker() {
            self.shell.log(Severity::Warning, reason);
            return;
        }
        let Some(section) = &self.section.last else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("zip");
        }
        let (import_id, section_id) = (section.import_id.clone(), section.section_id.clone());
        let target = path.clone();
        self.start(
            "Exporting DXF bundle",
            move |client| {
                let bytes = client.export_dxf(&import_id, &section_id)?;
                archive::write_archive(&path, &bytes)?;
                Ok(())
            },
            move |state, ()| {
                state.shell.log(
                    Severity::Info,
                    format!("Saved DXF bundle: {}", target.display()),
                )
            },
        );
    }

    /// Selects a body (from the tree or a diagnostic) and shows the 2D drawing.
    pub fn focus_component(&mut self, id: &str, related: Option<String>) {
        self.model.select(Some(id.to_owned()));
        self.model.related = related;
        self.preview.mode = ViewportMode::TwoD;
    }
}

impl AppState {
    /// Opens a `.c2mproj`: re-imports its STEP file, then restores plane, hidden bodies and
    /// number format. Component IDs derive from the file hash, so unknown IDs are dropped.
    pub fn open_project(&mut self, path: PathBuf) {
        let project = match crate::project::read_project(&path)
            .and_then(|text| crate::project::parse_project(&text))
        {
            Ok(project) => project,
            Err(error) => {
                self.shell.log(Severity::Error, error);
                return;
            }
        };
        let step = project.step.path.clone();
        self.start(
            "Opening project",
            move |client| client.import_step(&step),
            move |state, summary| {
                let changed = summary.sha256 != project.step.sha256;
                let known = |id: &String| summary.components.iter().any(|c| &c.id == id);
                let hidden = project.hidden_component_ids.iter().flatten();
                let hidden = hidden.filter(|id| known(id)).cloned().collect();
                state.finish_import(summary);
                state.section.restore(PlaneSnapshot {
                    kind: crate::project::parse_kind(&project.plane.kind).unwrap_or_default(),
                    offset_mm: project.plane.offset_mm,
                    custom_normal: project.plane.normal_xyz,
                });
                state.model.hidden = hidden;
                state.shell.set_format(project.number_format);
                if changed {
                    let text = format!(
                        "The STEP file changed since the project was saved (SHA-256 mismatch for {}). Results may differ from the saved session.",
                        project.step.path
                    );
                    state.shell.log(Severity::Warning, text);
                } else {
                    state.shell.log(Severity::Info, "Project opened.");
                }
                state.shell.project_path = Some(path);
            },
        );
    }
}

impl AppState {
    /// Fetches the display mesh once per import, when the 3D view is shown (like the React
    /// `PreviewView` on mount). It is not a blocking operation: the viewport shows its status.
    pub fn ensure_preview(&mut self) {
        if self.preview.mode != ViewportMode::ThreeD {
            return;
        }
        let Some(import_id) = self.model.import_id().map(str::to_owned) else {
            return;
        };
        if self.preview.requested.as_deref() == Some(import_id.as_str()) {
            return;
        }
        let Some(tasks) = self.tasks.clone() else {
            return;
        };
        self.preview.requested = Some(import_id.clone());
        self.preview.begin_loading();
        let wanted = import_id.clone();
        tasks.run(
            move |client| client.preview(&import_id),
            move |state, result| {
                // A reply for an earlier import is stale.
                if state.preview.requested.as_deref() != Some(wanted.as_str()) {
                    return;
                }
                if let (Err(message), true) = (&result, state.preview.mesh.is_none()) {
                    state.shell.log(Severity::Error, message.clone());
                }
                if let Some(imported) = &state.model.imported {
                    let bounds = imported.bounds_mm.clone();
                    state.preview.finish(result, &bounds);
                }
            },
        );
    }
}

impl AppState {
    /// Sets a custom plane on the selected sheet body's own plane (normal and offset).
    pub fn align_plane_to_selected(&mut self) {
        let (Some(import_id), Some(body)) = (
            self.model.import_id().map(str::to_owned),
            self.model.selected.clone(),
        ) else {
            self.shell.log(
                Severity::Warning,
                "Select a body in the model tree or the 2D view first.",
            );
            return;
        };
        self.start(
            "Aligning plane to body",
            move |client| client.component_plane(&import_id, &body),
            |state, plane| {
                let now = crate::state::now_ms();
                state
                    .section
                    .set_custom(plane.normal_xyz, Some(plane.offset_mm), now);
                state.shell.log(
                    Severity::Info,
                    "Custom plane aligned to the selected body. Compute the section to cut it.",
                );
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;
    use crate::api::fake::serve;

    /// An `AppState` whose worker results are collected, to be applied by the test.
    fn harness(url: &str) -> (AppState, mpsc::Receiver<Update>) {
        let (tx, rx) = mpsc::channel();
        let tx = std::sync::Mutex::new(tx);
        let tasks = Tasks::new(Client::new(url, None), move |u| {
            tx.lock().unwrap().send(u).unwrap()
        });
        (
            AppState {
                tasks: Some(tasks),
                ..AppState::default()
            },
            rx,
        )
    }

    fn settle(state: &mut AppState, rx: &mpsc::Receiver<Update>) {
        rx.recv_timeout(Duration::from_secs(5))
            .expect("worker result")(state);
    }

    const IMPORT: &str = r#"{"import_id":"i1","path":"m.step","sha256":"x","source_unit":"mm","to_mm_scale":1.0,
        "bounds_mm":{"min_xyz":[0,0,0],"max_xyz":[10,20,30]},"component_count":1,
        "components":[{"id":"c1","source_id":null,"source_name":null,"display_name":"Rotor","export_name":"Rotor_1",
        "hierarchy_path":[],"source_color":null,"name_provenance":null,"color_provenance":null,"body_ordinal":0,
        "bounds_mm":{"min_xyz":[0,0,0],"max_xyz":[10,20,30]}}],"diagnostics":[]}"#;
    const SECTION: &str = r#"{"section_id":"s1","import_id":"i1","plane":{"kind":"XY","offset_mm":15.0},
        "components":[{"component_id":"c1","wires":[{"closed":true,"role":"outer","curves":[{"type":"line","start":[0,0],"end":[1,0]}]}]}],
        "diagnostics":[{"code":"gap","severity":"warning","message":"gap","component_id":"c1"}]}"#;

    #[test]
    fn import_section_export_slice_updates_state_and_writes_the_bundle() {
        let fake = serve(vec![
            (200, "application/json", IMPORT.as_bytes().to_vec()),
            (200, "application/json", SECTION.as_bytes().to_vec()),
            (200, "application/zip", b"PK\x03\x04bundle".to_vec()),
        ]);
        let (mut state, rx) = harness(&fake.url);

        state.open_step(PathBuf::from("C:/m.step"));
        assert_eq!(state.shell.busy.as_deref(), Some("Importing STEP"));
        settle(&mut state, &rx);
        assert!(state.shell.busy.is_none());
        assert_eq!(state.model.visible_bodies().len(), 1);

        state.compute_section();
        settle(&mut state, &rx);
        assert_eq!(
            state.section.stats,
            Some(SectionStats {
                regions: 1,
                errors: 0,
                warnings: 1
            })
        );
        assert!(
            state
                .shell
                .messages
                .iter()
                .any(|m| m.text == "gap" && m.component_id.as_deref() == Some("c1"))
        );

        let dir = std::env::temp_dir().join(format!("c2m-slice-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        state.export_section(dir.join("out"));
        settle(&mut state, &rx);
        assert_eq!(
            std::fs::read(dir.join("out.zip")).unwrap(),
            b"PK\x03\x04bundle"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_import_is_reported_and_non_step_files_never_reach_the_service() {
        let body = br#"{"error":{"code":"bad","message":"Not a STEP file."}}"#.to_vec();
        let fake = serve(vec![(422, "application/json", body)]);
        let (mut state, rx) = harness(&fake.url);
        state.open_step(PathBuf::from("a.dxf"));
        assert!(state.shell.busy.is_none());
        state.open_step(PathBuf::from("a.STP"));
        settle(&mut state, &rx);
        let last = state.shell.messages.last().unwrap();
        assert_eq!(
            (last.severity, last.text.as_str()),
            (Severity::Error, "Not a STEP file.")
        );
        assert!(state.model.imported.is_none());
    }

    #[test]
    fn cancelled_operation_result_is_dropped_and_the_next_one_still_runs() {
        let fake = serve(vec![
            (200, "application/json", IMPORT.as_bytes().to_vec()),
            (200, "application/json", IMPORT.as_bytes().to_vec()),
        ]);
        let (mut state, rx) = harness(&fake.url);
        state.open_step(PathBuf::from("a.step"));
        state.cancel_operation();
        assert!(state.shell.busy.is_none());
        settle(&mut state, &rx); // the cancelled import's result: ignored
        assert!(state.model.imported.is_none());
        state.open_step(PathBuf::from("b.step"));
        settle(&mut state, &rx);
        assert!(state.model.imported.is_some());
    }

    #[test]
    fn a_second_operation_is_refused_while_one_runs() {
        let fake = serve(vec![(200, "application/json", IMPORT.as_bytes().to_vec())]);
        let (mut state, rx) = harness(&fake.url);
        state.open_step(PathBuf::from("a.step"));
        state.open_step(PathBuf::from("b.step"));
        assert_eq!(
            state.shell.messages.last().unwrap().severity,
            Severity::Warning
        );
        settle(&mut state, &rx);
    }
}
