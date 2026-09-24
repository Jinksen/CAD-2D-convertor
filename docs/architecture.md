# Architecture

CAD2Maxwell uses three deliberately narrow layers:

```text
React/TypeScript UI -> Tauri/Rust host -> loopback FastAPI/Python geometry service
```

React owns rendering, selection, and editable workspace state. It consumes versioned DTOs and must not contain geometry algorithms or depend on OpenCascade types. Rust owns desktop lifecycle, native integration, and future Python process supervision; it stays thin. Python owns exact CAD topology, sectioning, validation, and export.

The backend exposes `GET /api/v1/health` and `POST /api/v1/imports/step`. It binds to `127.0.0.1`, and browser origins are restricted to the Tauri and local Vite origins. Future process startup will provide a random per-session token.

The import route accepts only an absolute local STEP/STP path. `ImportService` validates the path, fingerprints the file, and maps XCAF results into Pydantic DTOs. `geometry/step_importer.py` is the OpenCascade boundary: it traverses occurrences, retains located exact solids, and extracts source metadata. `ImportSessions` owns those solids in memory under a process-local `import_id`; a backend restart invalidates the ID. Neither Rust nor React receives an OpenCascade object.

The importer dependency is `cadquery-ocp-novtk`. It provides the exact OCP binding without VTK, which this backend-only slice does not use. Matching OCP stubs support backend type checking. Preview mesh generation remains a separate later consumer of the retained exact shapes.

Heavy geometry work will never run on the UI thread. Exact B-Rep objects remain backend-only; serializable Pydantic DTOs cross the process boundary.
