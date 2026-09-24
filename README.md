# CAD2Maxwell

CAD2Maxwell is a Windows desktop engineering preprocessor for turning exact 3D CAD assemblies into validated, traceable 2D geometry for ANSYS Maxwell 2D.

> Screenshot placeholder: the engineering workspace shell is implemented; CAD visualization is not yet connected.

## Status

Milestone 0 established the Tauri/React desktop shell and local FastAPI service. The first Milestone 1 backend slice now imports STEP/STP files into exact OpenCascade shapes and returns typed component metadata. Preview meshes and the React model tree are not yet connected.

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

## Known limitations

- STEP import is backend-only: no preview mesh, model tree, or selectable 3D viewer yet.
- Exact sectioning, validation, and DXF export are not implemented.
- Import sessions are lost on backend restart; packaged OCP runtime compatibility is not yet verified.
- The shell still contains engineering workspace placeholders rather than geometry viewers.
- Windows is the only packaging target for the MVP.

## Roadmap

1. STEP preview meshes, model tree, and 3D selection.
2. Exact B-Rep sectioning with component ownership.
3. Geometry validation and clickable diagnostics.
4. Deterministic 1:1 DXF plus JSON manifest export.
5. Project save/load, recent files, and undo/redo.

Read [CAD2Maxwell_PROJECT_SPEC.md](CAD2Maxwell_PROJECT_SPEC.md) for the authoritative product and implementation requirements.
