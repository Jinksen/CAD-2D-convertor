//! Blocking HTTP client for the authenticated local geometry service. Call it from a worker
//! thread (see `tasks`); errors are the actionable messages shown to the user.

pub mod dto;

use std::time::Duration;

use dto::{
    ComponentPlane, Health, ImportSummary, PreviewMesh, PreviewMeshWire, SectionPlane,
    SectionResponse,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:8000";
const UNAVAILABLE: &str = "The local geometry service is unavailable.";
const MAX_RESPONSE_BYTES: u64 = 256 * 1024 * 1024;

pub struct Client {
    base: String,
    token: Option<String>,
    agent: ureq::Agent,
}

struct Reply {
    status: u16,
    content_type: String,
    body: Vec<u8>,
}

/// Percent-encodes a path segment like JS `encodeURIComponent` (ids may contain any character).
fn segment(raw: &str) -> String {
    raw.bytes()
        .map(|b| match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b'!'
            | b'\''
            | b'('
            | b')'
            | b'*' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The service's `{"error": {"message": ...}}` text, when present.
fn error_message(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    value
        .get("error")?
        .get("message")?
        .as_str()
        .map(str::to_owned)
}

impl Client {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(3)))
            .build()
            .into();
        Client {
            base: base_url.trim_end_matches('/').to_owned(),
            token,
            agent,
        }
    }

    /// Base URL from `CAD2MAXWELL_BACKEND_URL`, else the default local port.
    pub fn from_env(token: Option<String>) -> Self {
        Self::new(
            &std::env::var("CAD2MAXWELL_BACKEND_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.into()),
            token,
        )
    }

    fn call(&self, path: &str, body: Option<Value>, accept: &str) -> Result<Reply, String> {
        let url = format!("{}/api/v1/{path}", self.base);
        let outcome = match body {
            None => {
                let mut request = self.agent.get(&url).header("Accept", accept);
                if let Some(token) = &self.token {
                    request = request.header("X-Session-Token", token);
                }
                request.call()
            }
            Some(body) => {
                let mut request = self
                    .agent
                    .post(&url)
                    .header("Accept", accept)
                    .header("Content-Type", "application/json");
                if let Some(token) = &self.token {
                    request = request.header("X-Session-Token", token);
                }
                request.send(body.to_string().as_bytes())
            }
        };
        let mut response = outcome.map_err(|_| UNAVAILABLE.to_string())?;
        let content_type = response
            .headers()
            .get("Content-Type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BYTES)
            .read_to_vec()
            .map_err(|_| "The geometry service returned an invalid response.".to_string())?;
        Ok(Reply {
            status,
            content_type,
            body,
        })
    }

    /// Success body decoded as `T`, or the service's error message, or `failure` / `invalid`.
    fn decode<T: DeserializeOwned>(
        reply: &Reply,
        failure: &str,
        invalid: &str,
    ) -> Result<T, String> {
        if !(200..300).contains(&reply.status) {
            return Err(Self::failure(reply, failure));
        }
        serde_json::from_slice(&reply.body).map_err(|_| invalid.to_string())
    }

    fn failure(reply: &Reply, failure: &str) -> String {
        error_message(&reply.body).unwrap_or_else(|| {
            if reply.status == 401 {
                "The geometry service rejected the session token. Restart CAD2Maxwell.".into()
            } else {
                failure.into()
            }
        })
    }

    /// `Err` means offline or unusable, matching TS `fetchBackendHealth` (state: offline).
    pub fn health(&self) -> Result<Health, String> {
        let reply = self.call("health", None, "application/json")?;
        let health: Health = Self::decode(
            &reply,
            "The geometry service is not healthy.",
            "The geometry service returned an invalid health response.",
        )?;
        health
            .validate()
            .map_err(|_| "The geometry service returned an invalid health response.".to_string())?;
        Ok(health)
    }

    pub fn import_step(&self, path: &str) -> Result<ImportSummary, String> {
        let invalid = "The geometry service returned an invalid import response.";
        let reply = self.call(
            "imports/step",
            Some(json!({ "path": path })),
            "application/json",
        )?;
        let summary: ImportSummary =
            Self::decode(&reply, "The STEP file could not be imported.", invalid)?;
        summary.validate().map_err(|_| invalid.to_string())?;
        Ok(summary)
    }

    pub fn preview(&self, import_id: &str) -> Result<PreviewMesh, String> {
        let invalid = "The geometry service returned an invalid preview response.";
        let reply = self.call(
            &format!("imports/{}/preview", segment(import_id)),
            None,
            "application/json",
        )?;
        let wire: PreviewMeshWire =
            Self::decode(&reply, "The 3D preview could not be loaded.", invalid)?;
        if wire.import_id != import_id {
            return Err(invalid.into());
        }
        wire.decode().map_err(|_| invalid.to_string())
    }

    pub fn section(
        &self,
        import_id: &str,
        plane: &SectionPlane,
    ) -> Result<SectionResponse, String> {
        let invalid = "The geometry service returned an invalid section response.";
        let reply = self.call(
            "section",
            Some(json!({ "import_id": import_id, "plane": plane })),
            "application/json",
        )?;
        let section: SectionResponse =
            Self::decode(&reply, "The exact section could not be completed.", invalid)?;
        if section.import_id != import_id || section.plane != *plane || section.validate().is_err()
        {
            return Err(invalid.into());
        }
        Ok(section)
    }

    pub fn component_plane(
        &self,
        import_id: &str,
        component_id: &str,
    ) -> Result<ComponentPlane, String> {
        let path = format!(
            "imports/{}/components/{}/plane",
            segment(import_id),
            segment(component_id)
        );
        let reply = self.call(&path, None, "application/json")?;
        let plane: ComponentPlane = Self::decode(
            &reply,
            "The body plane could not be determined.",
            "The geometry service returned an invalid plane response.",
        )?;
        if plane.component_id != component_id {
            return Err("The geometry service returned an invalid plane response.".into());
        }
        Ok(plane)
    }

    /// The DXF bundle (ZIP bytes) of a computed section.
    pub fn export_dxf(&self, import_id: &str, section_id: &str) -> Result<Vec<u8>, String> {
        let body = json!({ "import_id": import_id, "section_id": section_id });
        let reply = self.call("export/dxf", Some(body), "application/zip")?;
        if !(200..300).contains(&reply.status) {
            return Err(Self::failure(&reply, "The DXF could not be exported."));
        }
        if !reply.content_type.contains("application/zip") {
            return Err("The geometry service returned an invalid DXF download.".into());
        }
        Ok(reply.body)
    }
}

#[cfg(test)]
pub(crate) mod fake {
    //! A one-shot-per-request HTTP server on an ephemeral port, for client and workflow tests.
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// `(request head + body, response)`: responds to each connection in turn with `responses`.
    pub struct Fake {
        pub url: String,
        pub requests: Arc<Mutex<Vec<String>>>,
    }

    pub fn serve(responses: Vec<(u16, &'static str, Vec<u8>)>) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&requests);
        std::thread::spawn(move || {
            for (status, content_type, body) in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut raw = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let n = stream.read(&mut chunk).unwrap_or(0);
                    raw.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&raw).into_owned();
                    let Some(split) = text.find("\r\n\r\n") else {
                        if n == 0 { break } else { continue }
                    };
                    let length = text[..split]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                        })
                        .unwrap_or(0);
                    if raw.len() >= split + 4 + length || n == 0 {
                        break;
                    }
                }
                seen.lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&raw).into_owned());
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        Fake { url, requests }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::serve;
    use super::*;
    use crate::api::dto::BoundsMm;

    const IMPORT: &str = r#"{"import_id":"i1","path":"C:/m.step","sha256":"ab","source_unit":"mm","to_mm_scale":1.0,
        "bounds_mm":{"min_xyz":[0,0,0],"max_xyz":[10,20,30]},"component_count":1,
        "components":[{"id":"c1","source_id":null,"source_name":"Rotor","display_name":"Rotor","export_name":"Rotor_1",
        "hierarchy_path":["Motor","Rotor"],"source_color":[255,0,0],"name_provenance":"product","color_provenance":"shape",
        "body_ordinal":0,"bounds_mm":{"min_xyz":[0,0,0],"max_xyz":[10,20,30]}}],
        "diagnostics":[{"code":"w","severity":"warning","message":"m","component_id":null}],"extra":1}"#;

    const SECTION: &str = r#"{"section_id":"s1","import_id":"i1","plane":{"kind":"XY","offset_mm":15.0},
        "components":[{"component_id":"c1","wires":[{"closed":true,"role":"outer","curves":[
        {"type":"line","start":[0,0],"end":[1,0]},
        {"type":"circle","center":[0,0],"radius":2.0,"x_axis":[1,0],"y_axis":[0,1],"start_parameter":0,"end_parameter":6.28}]}]}],
        "diagnostics":[{"code":"d","severity":"error","message":"open","component_id":"c1","related_component_id":null}]}"#;

    fn client(url: &str) -> Client {
        Client::new(url, Some("tok".into()))
    }

    #[test]
    fn imports_with_session_token_and_decodes_the_summary() {
        let fake = serve(vec![(200, "application/json", IMPORT.as_bytes().to_vec())]);
        let summary = client(&fake.url).import_step("C:/m.step").unwrap();
        assert_eq!(summary.components[0].source_color, Some([255, 0, 0]));
        assert_eq!(summary.diagnostics.len(), 1);
        let request = fake.requests.lock().unwrap()[0].to_lowercase();
        assert!(request.starts_with("post /api/v1/imports/step"));
        assert!(request.contains("x-session-token: tok"));
        assert!(request.contains(r#"{"path":"c:/m.step"}"#));
    }

    #[test]
    fn section_round_trips_plane_and_decodes_curves() {
        let fake = serve(vec![(200, "application/json", SECTION.as_bytes().to_vec())]);
        let plane = SectionPlane::XY { offset_mm: 15.0 };
        let section = client(&fake.url).section("i1", &plane).unwrap();
        assert_eq!(section.components[0].wires[0].curves.len(), 2);
        assert!(
            fake.requests.lock().unwrap()[0].contains(r#""plane":{"kind":"XY","offset_mm":15.0}"#)
        );
    }

    #[test]
    fn section_for_another_plane_is_rejected() {
        let fake = serve(vec![(200, "application/json", SECTION.as_bytes().to_vec())]);
        let err = client(&fake.url)
            .section("i1", &SectionPlane::XY { offset_mm: 1.0 })
            .unwrap_err();
        assert_eq!(
            err,
            "The geometry service returned an invalid section response."
        );
    }

    #[test]
    fn unauthorized_without_message_tells_the_user_what_to_do() {
        let fake = serve(vec![(401, "application/json", b"{}".to_vec())]);
        let err = client(&fake.url).health().unwrap_err();
        assert!(err.contains("session token"), "{err}");
    }

    #[test]
    fn service_error_message_is_passed_through() {
        let body = br#"{"error":{"code":"unknown_import","message":"Unknown import."}}"#.to_vec();
        let fake = serve(vec![(404, "application/json", body)]);
        assert_eq!(
            client(&fake.url).import_step("x.step").unwrap_err(),
            "Unknown import."
        );
    }

    #[test]
    fn malformed_json_and_wrong_shapes_are_explicit_errors() {
        let fake = serve(vec![
            (200, "application/json", b"not json".to_vec()),
            (
                200,
                "application/json",
                br#"{"status":"bad","service":"s","api_version":"1"}"#.to_vec(),
            ),
            (
                200,
                "application/json",
                IMPORT
                    .replace("\"component_count\":1", "\"component_count\":2")
                    .into_bytes(),
            ),
        ]);
        let c = client(&fake.url);
        assert_eq!(
            c.import_step("x").unwrap_err(),
            "The geometry service returned an invalid import response."
        );
        assert_eq!(
            c.health().unwrap_err(),
            "The geometry service returned an invalid health response."
        );
        assert_eq!(
            c.import_step("x").unwrap_err(),
            "The geometry service returned an invalid import response."
        );
    }

    #[test]
    fn export_requires_a_zip_response() {
        let fake = serve(vec![
            (200, "application/zip", b"PK\x03\x04zz".to_vec()),
            (200, "text/plain", b"nope".to_vec()),
        ]);
        let c = client(&fake.url);
        assert_eq!(c.export_dxf("i1", "s1").unwrap(), b"PK\x03\x04zz");
        assert_eq!(
            c.export_dxf("i1", "s1").unwrap_err(),
            "The geometry service returned an invalid DXF download."
        );
    }

    #[test]
    fn unreachable_service_is_reported_as_unavailable() {
        let url = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://{}", l.local_addr().unwrap())
        };
        assert_eq!(client(&url).health().unwrap_err(), UNAVAILABLE);
    }

    #[test]
    fn preview_buffers_are_decoded_and_validated() {
        use base64::Engine;
        let enc = |bytes: Vec<u8>| base64::engine::general_purpose::STANDARD.encode(bytes);
        let f = |v: &[f32]| enc(v.iter().flat_map(|x| x.to_le_bytes()).collect());
        let u = |v: &[u32]| enc(v.iter().flat_map(|x| x.to_le_bytes()).collect());
        let mesh = |indices: &[u32]| PreviewMeshWire {
            import_id: "i1".into(),
            deflection_mm: 0.1,
            coarsened: false,
            components: vec![dto::PreviewComponentWire {
                component_id: "c1".into(),
                color: None,
                positions: f(&[0., 0., 0., 1., 0., 0., 0., 1., 0.]),
                normals: f(&[0., 0., 1., 0., 0., 1., 0., 0., 1.]),
                indices: u(indices),
                edge_indices: u(&[0, 1]),
            }],
        };
        assert_eq!(
            mesh(&[0, 1, 2]).decode().unwrap().components[0].indices,
            vec![0, 1, 2]
        );
        assert!(mesh(&[0, 1, 3]).decode().is_err());
    }

    /// Request line and JSON body the client sent, to compare with the TS clients
    /// (`frontend/src/features/*/client.ts`) for the same user action.
    fn sent(fake: &fake::Fake, n: usize) -> (String, Value) {
        let raw = fake.requests.lock().unwrap()[n].clone();
        let (head, body) = raw.split_once("\r\n\r\n").unwrap();
        let body = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(body).unwrap()
        };
        (head.to_owned(), body)
    }

    #[test]
    fn requests_match_the_ts_clients() {
        use crate::state::section::{PlaneKind, SectionState};
        let fake = serve(
            (0..5)
                .map(|_| (500, "application/json", b"{}".to_vec()))
                .collect(),
        );
        let c = client(&fake.url);
        let bounds = BoundsMm {
            min_xyz: [0.0; 3],
            max_xyz: [10.0, 20.0, 30.0],
        };
        let mut state = SectionState::default();
        let _ = c.import_step(r"C:\models\Motor.step");
        let _ = c.preview("imp 1/\u{fc}!'()*~");
        let _ = c.component_plane("imp-1", "body 1");
        let _ = c.export_dxf("imp-1", "sec-1");
        let _ = c.section("imp-1", &state.request(&bounds).unwrap());

        let (head, body) = sent(&fake, 0);
        assert!(
            head.starts_with("POST /api/v1/imports/step HTTP/1.1"),
            "{head}"
        );
        assert_eq!(body, json!({"path": r"C:\models\Motor.step"}));
        // encodeURIComponent("imp 1/\u{fc}!'()*~")
        let (head, _) = sent(&fake, 1);
        assert!(
            head.starts_with("GET /api/v1/imports/imp%201%2F%C3%BC!'()*~/preview "),
            "{head}"
        );
        let (head, _) = sent(&fake, 2);
        assert!(
            head.starts_with("GET /api/v1/imports/imp-1/components/body%201/plane "),
            "{head}"
        );
        let (head, body) = sent(&fake, 3);
        assert!(head.starts_with("POST /api/v1/export/dxf "), "{head}");
        assert!(head.to_lowercase().contains("accept: application/zip"));
        assert_eq!(body, json!({"import_id": "imp-1", "section_id": "sec-1"}));
        // Default plane: XY through the model midpoint (z = 15).
        let (head, body) = sent(&fake, 4);
        assert!(head.starts_with("POST /api/v1/section "), "{head}");
        assert_eq!(
            body,
            json!({"import_id": "imp-1", "plane": {"kind": "XY", "offset_mm": 15.0}})
        );

        // Planes, with the JSON the TS `sectionPlaneRequest` produced (run with node on
        // plane.ts): numbers must be bit-identical.
        for (kind, name, offset) in [
            (PlaneKind::XY, "XY", 0.1 + 0.2),
            (PlaneKind::XZ, "XZ", -3.5),
            (PlaneKind::YZ, "YZ", 1e-7),
        ] {
            state.plane = kind;
            state.offset_mm = Some(offset);
            let plane = serde_json::to_value(state.request(&bounds).unwrap()).unwrap();
            assert_eq!(plane, json!({"kind": name, "offset_mm": offset}));
        }
        let custom = |n: [f64; 3], o: f64, normal: [f64; 3], x: [f64; 3], origin: [f64; 3]| {
            let mut s = SectionState::default();
            s.plane = PlaneKind::Custom;
            s.custom_normal = n;
            s.offset_mm = Some(o);
            let want = json!({"kind": "custom", "normal_xyz": normal, "x_dir_xyz": x, "origin_xyz": origin});
            assert_eq!(
                serde_json::to_value(s.request(&bounds).unwrap()).unwrap(),
                want,
                "{n:?}"
            );
        };
        custom(
            [0.331, 0.013, 0.944],
            12.5,
            [0.3308567700211607, 0.01299437465339906, 0.9435915132929779],
            [
                0.9436809830293099,
                -0.0045558582864172435,
                -0.33082540172137526,
            ],
            [4.135709625264509, 0.16242968316748824, 11.794893916162223],
        );
        custom(
            [1.0, 2.0, 3.0],
            -7.25,
            [0.2672612419124244, 0.5345224838248488, 0.8017837257372733],
            [
                0.9636241116594316,
                -0.14824986333222026,
                -0.2223747949983304,
            ],
            [-1.9376440038650768, -3.8752880077301537, -5.812932011595231],
        );
        custom(
            [0.1, 0.2, 0.3],
            0.1,
            [0.2672612419124244, 0.5345224838248488, 0.8017837257372731],
            [
                0.9636241116594316,
                -0.14824986333222026,
                -0.22237479499833035,
            ],
            [
                0.026726124191242442,
                0.053452248382484885,
                0.08017837257372731,
            ],
        );
        custom(
            [3.0, 4.0, 12.0],
            1.0 / 3.0,
            [0.23076923076923078, 0.3076923076923077, 0.9230769230769231],
            [0.9730085108210399, -0.072975638311578, -0.218926914934734],
            [0.07692307692307693, 0.10256410256410256, 0.3076923076923077],
        );
        custom(
            [-0.7, 0.7, 0.0001],
            250.0,
            [
                -0.7071067775788599,
                0.7071067775788599,
                0.00010101525393983714,
            ],
            [
                0.7071067847942352,
                0.7071067703634847,
                0.00010101525290906925,
            ],
            [
                -176.77669439471498,
                176.77669439471498,
                0.025253813484959284,
            ],
        );
    }

    /// `cargo test -- --ignored` with a running service: CAD2MAXWELL_SESSION_TOKEN and CAD2MAXWELL_TEST_STEP.
    #[test]
    #[ignore = "needs the running geometry service"]
    fn real_service_import_section_export() {
        let step = std::env::var("CAD2MAXWELL_TEST_STEP").expect("CAD2MAXWELL_TEST_STEP");
        let c = Client::from_env(std::env::var("CAD2MAXWELL_SESSION_TOKEN").ok());
        c.health().unwrap();
        let summary = c.import_step(&step).unwrap();
        let mut state = crate::state::section::SectionState::default();
        let plane = state.request(&summary.bounds_mm).unwrap();
        let section = c.section(&summary.import_id, &plane).unwrap();
        state.last = Some(section.clone());
        let zip = c
            .export_dxf(&summary.import_id, &section.section_id)
            .unwrap();
        assert!(zip.starts_with(b"PK\x03\x04"));
        let mesh = c.preview(&summary.import_id).unwrap();
        println!(
            "bodies {} curves {:?} zip {} preview comps {}",
            summary.component_count,
            section
                .components
                .iter()
                .map(|c| c.wires.len())
                .sum::<usize>(),
            zip.len(),
            mesh.components.len()
        );
        let first = &summary.components[0];
        println!(
            "plane {:?}",
            c.component_plane(&summary.import_id, &first.id)
        );
    }

    /// Writes `rust_{XY,XZ,YZ}.zip` into `CAD2MAXWELL_PARITY_OUT` using the app's request path
    /// (default plane offsets); a script sends the TS client's requests and diffs the archives.
    #[test]
    #[ignore = "needs the running geometry service"]
    fn real_service_export_per_plane() {
        use crate::state::section::{PlaneKind, SectionState};
        let step = std::env::var("CAD2MAXWELL_TEST_STEP").expect("CAD2MAXWELL_TEST_STEP");
        let out = std::path::PathBuf::from(
            std::env::var("CAD2MAXWELL_PARITY_OUT").expect("CAD2MAXWELL_PARITY_OUT"),
        );
        let c = Client::from_env(std::env::var("CAD2MAXWELL_SESSION_TOKEN").ok());
        let summary = c.import_step(&step).unwrap();
        for (kind, name) in [
            (PlaneKind::XY, "XY"),
            (PlaneKind::XZ, "XZ"),
            (PlaneKind::YZ, "YZ"),
        ] {
            let mut state = SectionState::default();
            state.plane = kind;
            let plane = state.request(&summary.bounds_mm).unwrap();
            let section = c.section(&summary.import_id, &plane).unwrap();
            let zip = c
                .export_dxf(&summary.import_id, &section.section_id)
                .unwrap();
            std::fs::write(out.join(format!("rust_{name}.zip")), zip).unwrap();
        }
    }
}
