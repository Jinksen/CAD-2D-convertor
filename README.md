# CAD2Maxwell

CAD2Maxwell is a Windows desktop engineering preprocessor for turning exact 3D CAD assemblies into validated, traceable 2D geometry for ANSYS Maxwell 2D.

> Screenshot placeholder: STEP metadata and exact 2D section outlines are connected; the 3D preview is still pending.

## Status

Milestone 0 established the Tauri/React desktop shell and local FastAPI service. The desktop workspace imports STEP/STP files into exact OpenCascade shapes, shows component metadata, and computes exact 2D section wires for XY, XZ, and YZ planes. The backend also accepts a custom plane. A command-line draft DXF exporter preserves lines, circles, arcs, and ellipses with a traceability manifest. Preview meshes and UI export are not yet connected.

## Supported files

The backend accepts local `.step` and `.stp` files. `.c2mproj` project files and DXF export follow in later milestones.

## Prerequisites

- Windows 11
- Git
- Python 3.12 or newer and [uv](https://docs.astral.sh/uv/)
- Node.js current LTS with Corepack
- Rust stable
- Visual Studio 2022 Build Tools with **Desktop development with C++**
- WebView2 Runtime

Use a Developer PowerShell so `link.exe` is available on `PATH`.

## Setup

```powershell
.\scripts\setup.ps1
```

The script validates prerequisites, installs the pinned pnpm workspace, and synchronizes the Python environment.

## Development

```powershell
.\scripts\dev.ps1
```

This starts the FastAPI service on `127.0.0.1:8000`, waits for its health endpoint, and launches Tauri development mode. Child backend processes are stopped when the desktop process exits.

## Test

```powershell
.\scripts\test.ps1
```

The quality gate runs frontend lint/typecheck/tests, backend Ruff/mypy/pytest, and Rust fmt/Clippy/tests.

## Package

```powershell
.\scripts\package.ps1
```

Packaging first runs the full quality gate and then invokes the Tauri bundle build.

## Architecture

- React and TypeScript own presentation and workspace state.
- Rust is a thin Tauri lifecycle/process bridge.
- Python is the future geometry authority and exposes versioned typed DTOs.
- Python now imports exact STEP B-Rep bodies through XCAF, retaining shapes in process-local sessions.
- The backend binds only to loopback; no CAD data is uploaded or telemetry collected.

See [architecture](docs/architecture.md), [geometry](docs/geometry.md), [DXF export](docs/dxf-export.md), and [UX](docs/ux.md).

## STEP import API

With the backend running, send an absolute local path:

```http
POST /api/v1/imports/step
Content-Type: application/json

{"path":"C:/Projects/Motor/assembly.step"}
```

The response includes a process-local `import_id`, SHA-256 source fingerprint, source unit and millimetre conversion scale, model bounds, ordered components, and diagnostics. Components carry stable IDs, separate source/display/export names, hierarchy, color where present, and millimetre bounds. Invalid paths return 422; missing, unreadable, and rejected STEP files return 404, 403, and 400 respectively. See [geometry](docs/geometry.md) for coordinate and metadata behavior.

## Exact section API

After importing a file, use its `import_id` to request a section:

```http
POST /api/v1/section
Content-Type: application/json

{"import_id":"<import_id>","plane":{"kind":"XY","offset_mm":110.0}}
```

`XY` offsets are Z coordinates, `XZ` offsets are Y coordinates, and `YZ` offsets are X coordinates, all in millimetres. A custom plane uses `kind: "custom"` with `origin_xyz`, perpendicular `normal_xyz` and `x_dir_xyz`. The response returns component-owned closed/open wires and exact line, circle, and ellipse parameters. It reports an empty section or open wires explicitly. Unsupported curve types fail the request instead of being silently flattened. Section IDs and exact wires are process-local.

## Draft DXF conversion

From the repository root, after setup:

```powershell
uv run --project backend python -m cad2maxwell_backend.cli `
  --input "C:\Projects\Motor\assembly.step" `
  --plane XY --offset-mm 110.0 `
  --output "C:\Projects\Motor\section.dxf"
```

The output path must be absolute and unused. The command writes `section.dxf` and `section.json`; it never overwrites existing files. The DXF uses millimetres and per-component ASCII-safe layers. The manifest records source identity, diagnostics, and outer/hole wire roles. Its `draft_unvalidated` status means cross-component overlaps and a real Maxwell import have not been checked.

## Known limitations

- STEP import currently requires pasting an absolute local path; a native file picker is not yet connected.
- The model tree lists and selects components and the 2D viewport shows exact section curves, but there is no preview mesh or selectable 3D viewer yet.
- Closed section wires are classified as outer or hole by exact containment, but self-intersections and cross-component overlaps are not yet validated. Draft DXF export is CLI-only and has not been tested in ANSYS Maxwell.
- The UI supports XY/XZ/YZ offsets; custom planes are available through the backend API.
- Import sessions are lost on backend restart; packaged OCP runtime compatibility is not yet verified.
- The 3D viewport remains a placeholder.
- Windows is the only packaging target for the MVP.

## Roadmap

1. Native STEP file picker, preview meshes, and 3D selection.
2. Region and hole classification, geometry validation, and clickable diagnostics.
3. Complete DXF validation, Maxwell import verification, and UI export.
4. Project save/load, recent files, and undo/redo.

Read [CAD2Maxwell_PROJECT_SPEC.md](CAD2Maxwell_PROJECT_SPEC.md) for the authoritative product and implementation requirements.
