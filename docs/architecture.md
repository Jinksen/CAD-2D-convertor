# Architecture

## Current production implementation

```text
Rune native UI (Rust) -> loopback FastAPI/Python geometry service
```

The Rune app (`frontend/rune-app`) owns rendering, selection, and editable workspace state. It decodes versioned DTOs into typed Rust structs and must not contain geometry algorithms or depend on OpenCascade types; the UI keeps the previous model if a later import fails. It also owns window, dialog and service-process lifecycle, and stays a thin adapter. Python owns exact CAD topology, sectioning, validation, and export.

The backend exposes health, absolute-path STEP import, streamed STEP file import, a preview mesh, exact section, and draft DXF archive routes. It binds to `127.0.0.1`, and requests from unapproved browser origins are rejected before routing. The local launchers generate a random per-session token required on all routes, including health. The app generates the token and passes it to the service it spawns (`backend/cad2maxwell-backend.exe` next to the app executable); when `CAD2MAXWELL_SESSION_TOKEN` is already set (dev and trial launchers) it uses that service and spawns nothing. Allowed CORS preflight requests do not require the token. Direct API development without a configured token remains available.

`START-APP.cmd` invokes `scripts/run.ps1` to supervise the backend and compiled local trial. `scripts/build-trial.ps1` builds the release app into `artifacts/CAD2Maxwell`; Rust is a build prerequisite, not launch requirements. The trial still depends on the checkout's Python environment. The launcher refuses occupied ports, checks authenticated service identity, logs startup output, and stops only the processes it starts. Standalone runtime bundling remains future work.

Rust provides native ZIP saving through `rfd` 0.16.0, an MIT-licensed native-dialog crate. The save path accepts generated archive bytes and obtains its output path from the native dialog, refuses overwrites and non-ZIP filenames, limits saves to 64 MiB, and removes partially written files on write failure. Python remains the sole geometry and DXF authority.

The import route accepts only an absolute local STEP/STP path. `ImportService` validates the path, fingerprints the file, and maps XCAF results into Pydantic DTOs. `geometry/step_importer.py` is the OpenCascade boundary: it traverses occurrences, retains located exact solids, and extracts source metadata. `ImportSessions` owns those solids in memory under a process-local `import_id`; a backend restart invalidates the ID. The Rust app never receives an OpenCascade object.

The importer dependency is `cadquery-ocp-novtk`. It provides the exact OCP binding without VTK. Matching OCP stubs support backend type checking. The preview route triangulates retained shapes into a typed, capped display DTO. The app rasterizes those triangles on the CPU into a `rune::Pixels` buffer (Rune has no 3D API) with id-buffer picking, while Python keeps all exact B-Rep objects.

The section service reads exact solids from the import session, intersects each with the requested plane, and retains assembled OpenCascade wires in a separate process-local section session. A typed DTO carries analytic 2D curves and component IDs to the app. It draws those curves as Rune shapes, keeping arcs analytic and tessellating only for display; it does not compute intersections or alter topology.

The draft DXF exporter consumes typed import and section results, writes analytic modelspace entities through ezdxf, and emits a traceability manifest. The CLI runs import, section, and export in one process; the UI downloads a ZIP built from process-local session summaries. Section validation blocks export for positive-area cross-component overlaps and invalid planar faces.

Heavy geometry work will never run on the UI thread. Exact B-Rep objects remain backend-only; serializable Pydantic DTOs cross the process boundary.

## Desktop architecture

The desktop UI is a native Rune/Rust application; Python/OpenCascade stays the geometry authority:

```text
Rune native UI -> Rust desktop/service adapter -> authenticated loopback FastAPI -> Python/OpenCascade
```

Gates, measured results and known gaps are in [rune-migration.md](rune-migration.md).
