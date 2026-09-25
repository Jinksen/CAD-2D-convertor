# CAD2Maxwell

CAD2Maxwell is a Windows desktop engineering preprocessor for turning exact 3D CAD assemblies into validated, traceable 2D geometry for ANSYS Maxwell 2D.

> Screenshot placeholder: STEP metadata, 3D preview, and exact 2D section outlines are connected.

## Status

Milestone 0 established the Tauri/React desktop shell and local FastAPI service. The desktop workspace imports STEP/STP files into exact OpenCascade shapes, shows component metadata and a selectable 3D preview, and computes exact 2D section wires for XY, XZ, and YZ planes. The backend also accepts a custom plane. The UI and command line export draft analytic DXF with a traceability manifest.

## Supported files

The backend accepts local `.step` and `.stp` files and exports draft DXF. `.c2mproj` project files follow in a later milestone.

## Prerequisites

- Windows 11
- Git
- Python 3.12 or newer and [uv](https://docs.astral.sh/uv/)
- Node.js current LTS with Corepack
- Rust stable
- Visual Studio 2022 Build Tools with **Desktop development with C++**
- WebView2 Runtime

The scripts locate the installed MSVC linker automatically, including from a regular PyCharm PowerShell terminal.

## Setup

```powershell
.\scripts\setup.ps1
```

The script validates prerequisites, installs the pinned pnpm workspace, and synchronizes the Python environment.

## Development

On Windows, double-click `START-APP.cmd` in the repository folder to open the desktop app. Keep the console window open while using it.

In PyCharm, open the **Terminal** tab at the repository root and run this in PowerShell:

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
- Python is the geometry authority and exposes versioned typed DTOs.
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

## Try the desktop workflow

Run `./scripts/dev.ps1`, choose a `.step` or `.stp` file with **Choose STEP file**, select XY/XZ/YZ, and click **Compute Section**. The initial offset is the midpoint of the model bounds; change it in millimetres as needed. When the section has closed, classified wires, click **Export draft DXF**. The app downloads `section.zip` containing `section.dxf` and `section.json`. The file chooser sends the selected file only to the local loopback geometry service and rejects uploads over 512 MiB. You can still paste an absolute path and use **Import STEP**.

After import, use **3D VIEW** to orbit with the mouse, zoom with the wheel, and click a body to select it. The backend triangulates retained exact shapes for display only. Switching back to **2D SECTION** keeps the current section visible. The preview currently uses JSON transport with a 50,000-triangle limit; a larger model reports a preview error while exact sectioning remains available.

The ZIP export uses the backend's retained import and section sessions. If the backend restarts, import and compute again. Exact positive-area overlaps between components block export and identify both components. Invalid planar faces, including self-intersecting contours, also block export. The draft is not yet a fully validated simulation model: duplicate and tiny-edge checks and complex-model Maxwell verification remain pending.

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

- STEP import supports the system file chooser and absolute-path entry. File chooser uploads are limited to 512 MiB.
- The 3D preview is selectable and linked to the model tree. Large models exceeding 50,000 preview triangles need a future binary mesh transport.
- Closed section wires are classified as outer or hole by exact containment. Positive-area overlaps between components and invalid planar faces block export. Duplicate and tiny-edge checks are still pending. A simple 10 by 20 mm rectangle DXF was imported as a filled region in ANSYS Maxwell; complex assemblies still need verification.
- The UI supports XY/XZ/YZ offsets; custom planes are available through the backend API.
- Import sessions are lost on backend restart; packaged OCP runtime compatibility is not yet verified.
- Windows is the only packaging target for the MVP.

## Roadmap

1. Complete duplicate/tiny-edge validation and complex-model Maxwell import verification.
2. Project save/load, recent files, and undo/redo.
3. Binary preview mesh transport for large assemblies and custom-plane controls in the UI.

Read [CAD2Maxwell_PROJECT_SPEC.md](CAD2Maxwell_PROJECT_SPEC.md) for the authoritative product and implementation requirements.
