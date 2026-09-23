# CAD2Maxwell

CAD2Maxwell is a Windows desktop engineering preprocessor for turning exact 3D CAD assemblies into validated, traceable 2D geometry for ANSYS Maxwell 2D.

> Screenshot placeholder: the Milestone 0 engineering workspace shell is implemented; CAD visualization arrives in Milestone 1.

## Status

Milestone 0 establishes the Tauri/React desktop shell, typed local FastAPI health service, tests, and Windows developer workflows. STEP import and geometry processing are not implemented yet.

## Supported files

No CAD files are processed in Milestone 0. STEP/STP is the first planned import format; `.c2mproj` project files and DXF export follow in later milestones.

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
- The backend binds only to loopback; no CAD data is uploaded or telemetry collected.

See [architecture](docs/architecture.md), [geometry](docs/geometry.md), [DXF export](docs/dxf-export.md), and [UX](docs/ux.md).

## Known limitations

- No STEP import, OpenCascade integration, sectioning, validation, or DXF export yet.
- The backend health check is the only API operation.
- The shell contains engineering workspace placeholders rather than geometry viewers.
- Windows is the only packaging target for the MVP.

## Roadmap

1. STEP import, metadata extraction, and preview meshes.
2. Exact B-Rep sectioning with component ownership.
3. Geometry validation and clickable diagnostics.
4. Deterministic 1:1 DXF plus JSON manifest export.
5. Project save/load, recent files, and undo/redo.

Read [CAD2Maxwell_PROJECT_SPEC.md](CAD2Maxwell_PROJECT_SPEC.md) for the authoritative product and implementation requirements.
