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

/// Extensions of the CAD formats the service imports.
pub const MODEL_EXTENSIONS: [&str; 5] = ["step", "stp", "igs", "iges", "brep"];

fn is_model(path: &Path) -> bool {
    path.extension().is_some_and(|e| {
        MODEL_EXTENSIONS
            .iter()
            .any(|known| e.eq_ignore_ascii_case(known))
    })
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
        if !is_model(&path) {
            self.shell.log(
                Severity::Error,
                "Choose a STEP, IGES or BREP file (.step .stp .igs .iges .brep).",
            );
            return;
        }
        let service_path = path.to_string_lossy().into_owned();
        self.start(
            "Importing model",
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
            self.shell.push(Message {
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
        if let Some(error) = self.section.tolerance.error.iter().flatten().next() {
            let text = format!("Fix the advanced tolerances first: {error}");
            self.shell.log(Severity::Error, text);
            return;
        }
        let Some(imported) = &self.model.imported else {
            self.shell.log(
                Severity::Warning,
                "Open a model before computing a section.",
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
        let tolerance = self.section.tolerance.request();
        self.start(
            "Computing exact section",
            move |client| client.section(&import_id, &plane, tolerance.as_ref()),
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
            "Section {}: {} closed outer regions, {} errors, {} warnings.",
            describe_plane(sent, &self.shell.number_format),
            stats.regions,
            stats.errors,
            stats.warnings
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
            self.shell.push(Message {
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
        let zip = self.shell.settings.export.zip;
        if zip && path.extension().is_none() {
            path.set_extension("zip");
        } else if !zip {
            path.set_extension(""); // the bundle becomes a folder of this name
        }
        let (import_id, section_id) = (section.import_id.clone(), section.section_id.clone());
        let options = self.export_options();
        let target = path.clone();
        self.start(
            "Exporting DXF bundle",
            move |client| {
                let bytes = client.export_dxf(&import_id, &section_id, &options)?;
                if zip {
                    archive::write_archive(&path, &bytes)
                } else {
                    archive::extract_archive(&path, &bytes)
                }
            },
            move |state, ()| {
                state.shell.notify(
                    "DXF bundle exported",
                    format!("Saved DXF bundle: {}", target.display()),
                )
            },
        );
    }

    /// Frames one body in both the 3D and the 2D view (display only).
    pub fn zoom_to(&mut self, id: &str) {
        if let Some(imported) = &self.model.imported {
            self.preview.fit(Some(id), &imported.bounds_mm);
        }
        self.section.fit(Some(id));
    }

    /// Selects a body (from the tree or a diagnostic) and shows the 2D drawing.
    pub fn focus_component(&mut self, id: &str, related: Option<String>) {
        self.model.select(Some(id.to_owned()));
        self.model.related = related;
        self.preview.mode = ViewportMode::TwoD;
    }
}

/// "XY at z = 37.500 mm" or "custom, normal (0, 0, 1)", in the user's number format.
fn describe_plane(
    plane: &crate::api::dto::SectionPlane,
    fmt: &crate::format::NumberFormat,
) -> String {
    use crate::api::dto::SectionPlane::{Custom, XY, XZ, YZ};
    match plane {
        XY { offset_mm } => format!("XY at z = {}", fmt.length(*offset_mm)),
        XZ { offset_mm } => format!("XZ at y = {}", fmt.length(*offset_mm)),
        YZ { offset_mm } => format!("YZ at x = {}", fmt.length(*offset_mm)),
        Custom { normal_xyz, .. } => format!("custom, normal {}", fmt.direction(normal_xyz)),
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
        let resolved = project.step.resolve(&path);
        let moved = resolved.as_path() != Path::new(&project.step.path);
        let step = resolved.to_string_lossy().into_owned();
        let opened = step.clone();
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
                state
                    .section
                    .tolerance
                    .restore(project.tolerance.unwrap_or_default());
                state.model.hidden = hidden;
                state.model.restore_groups(project.groups);
                state.model.collapsed = project.collapsed_folders.into_iter().collect();
                state.shell.set_format(project.number_format);
                if moved {
                    let text = format!(
                        "{} was not found; opened {} next to the project instead.",
                        project.step.path, opened
                    );
                    state.shell.log(Severity::Info, text);
                }
                if changed {
                    let text = format!(
                        "The model file changed since the project was saved (SHA-256 mismatch for {}). Results may differ from the saved session.",
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
    /// Fetches the exact region metrics of the shown section when a body is selected, once per
    /// section id (cached). Not a blocking operation: the properties pane shows its status.
    /// Returns true when a request started.
    pub fn ensure_regions(&mut self) -> bool {
        let (Some(section), Some(_), Some(tasks)) = (
            self.section.last.as_ref(),
            self.model.selected.as_ref(),
            self.tasks.clone(),
        ) else {
            return false;
        };
        let id = section.section_id.clone();
        if !self.section.regions.begin(&id) {
            return false;
        }
        let wanted = id.clone();
        tasks.run(
            move |client| client.regions(&id).map(|r| r.regions),
            move |state, result| state.section.regions.finish(&wanted, result),
        );
        true
    }

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
        assert_eq!(state.shell.busy.as_deref(), Some("Importing model"));
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
    fn regions_are_fetched_once_per_section_after_a_body_is_selected() {
        let regions = br#"{"section_id":"s1","regions":[{"component_id":"c1","region_index":0,"outer_wire":0,
            "hole_wires":[],"area_mm2":12.0,"perimeter_mm":14.0,"centroid":[1,2],"hole_count":0,"interior_point":[1,2]}]}"#;
        let fake = serve(vec![
            (200, "application/json", IMPORT.as_bytes().to_vec()),
            (200, "application/json", SECTION.as_bytes().to_vec()),
            (200, "application/json", regions.to_vec()),
        ]);
        let (mut state, rx) = harness(&fake.url);
        state.open_step(PathBuf::from("C:/m.step"));
        settle(&mut state, &rx);
        state.compute_section();
        settle(&mut state, &rx);
        assert!(!state.ensure_regions(), "nothing selected, nothing fetched");
        state.model.select(Some("c1".into()));
        assert!(state.ensure_regions());
        assert!(!state.ensure_regions(), "already in flight");
        assert_eq!(
            state.section.regions.get("s1"),
            Some(&crate::state::section::regions::Load::Loading)
        );
        settle(&mut state, &rx);
        assert_eq!(state.section.regions.of_body("s1", "c1")[0].area_mm2, 12.0);
        assert!(!state.ensure_regions(), "cached");
        assert_eq!(fake.requests.lock().unwrap().len(), 3);
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

    #[test]
    fn every_supported_cad_extension_is_accepted_and_others_are_not() {
        for name in ["a.step", "a.STP", "a.igs", "a.IGES", "a.brep"] {
            assert!(is_model(Path::new(name)), "{name}");
        }
        for name in ["a.dxf", "a.c2mproj", "a"] {
            assert!(!is_model(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn export_sends_the_settings_and_can_write_a_folder_instead_of_a_zip() {
        use std::io::Write;
        let mut cursor = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut cursor);
        zip.start_file("section.dxf", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            b"0
EOF",
        )
        .unwrap();
        zip.finish().unwrap();
        let fake = serve(vec![
            (200, "application/json", IMPORT.as_bytes().to_vec()),
            (200, "application/json", SECTION.as_bytes().to_vec()),
            (200, "application/zip", cursor.into_inner()),
        ]);
        let (mut state, rx) = harness(&fake.url);
        state.shell.edit_settings(|s| {
            s.export.zip = false;
            s.export.formats = vec!["svg".into()];
            s.export.layer_mode = crate::state::settings::LayerMode::Group;
        });
        state.open_step(PathBuf::from("C:/m.step"));
        settle(&mut state, &rx);
        state.compute_section();
        settle(&mut state, &rx);
        let dir = std::env::temp_dir().join(format!("c2m-export-dir-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        state.export_section(dir.join("bundle.zip"));
        settle(&mut state, &rx);
        assert_eq!(
            std::fs::read(dir.join("bundle").join("section.dxf")).unwrap(),
            b"0
EOF"
        );
        let request = fake.requests.lock().unwrap()[2].clone();
        assert!(request.contains(r#""layer_mode":"group""#), "{request}");
        assert!(request.contains(r#""formats":["dxf","svg"]"#), "{request}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn tolerances_are_sent_only_when_set_and_a_rejected_entry_blocks_compute() {
        let fake = serve(vec![
            (200, "application/json", IMPORT.as_bytes().to_vec()),
            (200, "application/json", SECTION.as_bytes().to_vec()),
        ]);
        let (mut state, rx) = harness(&fake.url);
        state.open_step(PathBuf::from("C:/m.step"));
        settle(&mut state, &rx);
        state.section.tolerance.text[1].replace_text("0.9");
        state.compute_section();
        assert!(
            state.shell.busy.is_none(),
            "invalid tolerance blocks the run"
        );
        assert_eq!(
            state.shell.messages.last().unwrap().severity,
            Severity::Error
        );
        state.section.tolerance.text[1].replace_text("0.01");
        state.compute_section();
        settle(&mut state, &rx);
        let request = fake.requests.lock().unwrap()[1].clone();
        assert!(
            request.contains(r#""tolerance":{"sliver_ratio":0.01}"#),
            "{request}"
        );
    }
}
