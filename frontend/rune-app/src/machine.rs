//! Rotational symmetry hint and the explicit 1/N sector (service `machine` endpoints).

use crate::api::dto::{SectionResponse, Severity};
use crate::state::AppState;
use crate::state::section::SectionStats;

impl AppState {
    /// Asks the service for the symmetry of the shown section.
    pub fn detect_symmetry(&mut self) {
        let Some(id) = self.section.last.as_ref().map(|s| s.section_id.clone()) else {
            self.shell.log(
                Severity::Warning,
                "Compute a section before detecting symmetry.",
            );
            return;
        };
        let asked = id.clone();
        self.start(
            "Detecting symmetry",
            move |client| client.symmetry(&id),
            move |state, found| {
                let text = if found.order > 1 {
                    format!(
                        "Rotational symmetry of order {} found (sector {:.2} deg, confidence {:.0} %).",
                        found.order,
                        found.sector_angle_deg,
                        found.confidence * 100.0
                    )
                } else {
                    "No rotational symmetry found.".to_owned()
                };
                state.shell.log(Severity::Info, text);
                state.section.symmetry = Some((asked, found));
            },
        );
    }

    /// Creates the 1/N sector of the shown section as a new named section. Only ever called
    /// from the button: nothing detects and cuts on its own.
    pub fn create_sector(&mut self) {
        let (Some(section), Some((id, found))) =
            (self.section.last.clone(), self.section.symmetry.clone())
        else {
            return;
        };
        if found.order < 2 || id != section.section_id {
            self.shell.log(
                Severity::Warning,
                "Detect the symmetry of the shown section first.",
            );
            return;
        }
        let order = found.order;
        self.start(
            "Creating sector",
            move |client| client.sector(&section, order),
            move |state, sector| state.finish_sector(sector, order, &id),
        );
    }

    fn finish_sector(&mut self, sector: SectionResponse, order: u32, source: &str) {
        // The shown section may have changed while the service worked.
        if self.section.last.as_ref().map(|s| s.section_id.as_str()) != Some(source) {
            return;
        }
        let name = format!("Sector 1 of {order}");
        if self.section.add_named(&name).is_err() {
            let _ = self.section.add_named("");
        }
        self.section.stats = Some(SectionStats::of(&sector));
        self.section.last = Some(sector);
        self.shell.log(
            Severity::Info,
            format!("Created {name} as a new section. The cut does not rerun overlap validation."),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::Duration;

    use crate::api::Client;
    use crate::api::fake::serve;
    use crate::state::AppState;
    use crate::tasks::{Tasks, Update};

    const IMPORT: &str = r#"{"import_id":"i1","path":"m.step","sha256":"x","source_unit":"mm","to_mm_scale":1.0,
        "bounds_mm":{"min_xyz":[0,0,0],"max_xyz":[10,20,30]},"component_count":1,
        "components":[{"id":"c1","source_id":null,"source_name":null,"display_name":"Rotor","export_name":"Rotor_1",
        "hierarchy_path":[],"source_color":null,"name_provenance":null,"color_provenance":null,"body_ordinal":0,
        "bounds_mm":{"min_xyz":[0,0,0],"max_xyz":[10,20,30]}}],"diagnostics":[]}"#;
    const SECTION: &str = r#"{"section_id":"s1","import_id":"i1","plane":{"kind":"XY","offset_mm":15.0},
        "components":[{"component_id":"c1","wires":[{"closed":true,"role":"outer","curves":[{"type":"line","start":[0,0],"end":[1,0]}]}]}],
        "diagnostics":[]}"#;
    const SECTOR: &str = r#"{"section_id":"s2","import_id":"i1","plane":{"kind":"XY","offset_mm":15.0},
        "components":[{"component_id":"c1","wires":[{"closed":true,"role":"outer","curves":[{"type":"line","start":[0,0],"end":[1,0]}]}]}],
        "diagnostics":[]}"#;
    const SYMMETRY: &str =
        r#"{"order":4,"sector_angle_deg":90.0,"center":[0,0],"confidence":0.97}"#;

    #[test]
    fn detect_then_an_explicit_sector_becomes_a_new_named_section() {
        let fake = serve(vec![
            (200, "application/json", IMPORT.as_bytes().to_vec()),
            (200, "application/json", SECTION.as_bytes().to_vec()),
            (200, "application/json", SYMMETRY.as_bytes().to_vec()),
            (200, "application/json", SECTOR.as_bytes().to_vec()),
        ]);
        let (tx, rx) = mpsc::channel();
        let tx = std::sync::Mutex::new(tx);
        let tasks = Tasks::new(Client::new(&fake.url, None), move |u: Update| {
            tx.lock().unwrap().send(u).unwrap()
        });
        let mut s = AppState {
            tasks: Some(tasks),
            ..AppState::default()
        };
        let settle = |s: &mut AppState| rx.recv_timeout(Duration::from_secs(5)).expect("result")(s);
        s.open_step(PathBuf::from("C:/m.step"));
        settle(&mut s);
        s.create_sector();
        assert!(
            s.shell.busy.is_none(),
            "no sector before a section and a detection"
        );
        s.compute_section();
        settle(&mut s);
        s.create_sector();
        assert!(s.shell.busy.is_none(), "no sector without a detection");
        s.detect_symmetry();
        settle(&mut s);
        assert_eq!(s.section.symmetry.as_ref().map(|(_, f)| f.order), Some(4));
        assert_eq!(
            fake.requests.lock().unwrap().len(),
            3,
            "detection never cuts"
        );
        s.create_sector();
        settle(&mut s);
        assert_eq!(s.section.named.list.len(), 2);
        assert_eq!(s.section.active_name(), Some("Sector 1 of 4"));
        assert_eq!(s.section.last.as_ref().unwrap().section_id, "s2");
        assert!(s.section.stats.is_some());
        let request = fake.requests.lock().unwrap()[3].clone();
        assert!(
            request.starts_with("POST /api/v1/sections/s1/sector"),
            "{request}"
        );
        assert!(request.contains(r#"{"order":4}"#));
    }
}
