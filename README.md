# CAD2Maxwell

A lightweight Windows desktop tool that takes a 3D STEP model and gives you an exact 2D section as a DXF, ready for ANSYS Maxwell 2D.

No Inventor, no drawing setup, no extra modelling steps: open the STEP file, pick a plane and an offset, check the section, export.

- Exact B-Rep sectioning with OpenCascade (no mesh slicing, arcs stay arcs).
- Component names and colors are kept and mapped to ASCII-safe DXF layers.
- A JSON manifest traces every 2D region back to its source component.
- Runs fully offline; the geometry service listens only on `127.0.0.1`.

## Quick start

1. Complete the one-time [setup](#one-time-setup).
2. Start the app with **CAD2Maxwell** on the Desktop or in the Start menu (or double-click `START-APP.cmd` in this folder).
3. Click **Open STEP** and open a `.step` / `.stp` model. Its bodies appear in **Model Builder**; check them in **3D Preview**.
4. Choose a plane in the ribbon:
   - **XY**, **XZ**, or **YZ** with an offset in millimetres (Z, Y, or X coordinate). The default is the middle of the model.
   - **Custom** for a tilted plane, for example laminations that do not lie in XY. Select one sheet in Model Builder or 3D Preview, open the **Section** node, and click **Align to selected body**. The plane snaps to the sheet plane at mid-thickness. You can also type the normal and the offset along it.
5. Open the **Section** node in Model Builder and click **Compute Section**. Filled regions are closed contours; holes stay empty.
6. Click **Export draft DXF** and save a new ZIP. It contains `section.dxf` (1:1, mm) and `section.json`.
7. Import `section.dxf` into Maxwell 2D.

Close the app window to stop it. The minimized launcher console stops the geometry service automatically. Startup logs are written to `artifacts/logs`.

See the [walkthrough](docs/first-trial.md) for details and troubleshooting.

## One-time setup

Prerequisites (Windows 11):

- Git
- Python 3.12+ and [uv](https://docs.astral.sh/uv/)
- Node.js LTS with Corepack
- Rust stable
- Visual Studio 2022 Build Tools with **Desktop development with C++**
- WebView2 Runtime

From the repository root in PowerShell:

```powershell
.\scripts\setup.ps1             # install pinned frontend and Python dependencies
.\scripts\build-trial.ps1       # build artifacts/CAD2Maxwell/CAD2Maxwell.exe
.\scripts\install-shortcuts.ps1 # add CAD2Maxwell to the Desktop and Start menu
```

The shortcuts point at this checkout, so keep the repository folder where it is. After moving it, run `install-shortcuts.ps1` again. Remove the shortcuts with `.\scripts\install-shortcuts.ps1 -Remove`.

After `git pull`, run `setup.ps1` and `build-trial.ps1` again to refresh the app.

## Export checks

Export is blocked with an error diagnostic when the section has:

- open contours,
- invalid or self-intersecting planar faces,
- duplicate edges,
- positive-area overlaps between components.

Tiny edges are reported as warnings and left untouched. Nothing is silently repaired or merged. Existing files are never overwritten.

## Command line

The same conversion without the UI:

```powershell
uv run --project backend python -m cad2maxwell_backend.cli `
  --input "C:\Projects\Motor\assembly.step" `
  --plane XY --offset-mm 110.0 `
  --output "C:\Projects\Motor\section.dxf"
```

The output path must be absolute and unused. The command writes `section.dxf` and `section.json`.

## Known limitations

- Exports are marked `draft_unvalidated`. A simple rectangle imports into Maxwell as a filled region; complex assemblies still need verification.
- **Align to selected body** uses the body's largest planar face family. It needs a body with planar faces, such as a sheet.
- The 3D preview is limited to 50,000 triangles. Sectioning still works for larger models.
- No project save/load yet. Re-import after restarting the app.
- Port 8000 must be free, so only one instance runs at a time.
- The app runs from this checkout's Python environment. It is not a standalone installer for other computers.

## Development

```powershell
.\scripts\dev.ps1    # hot-reload desktop app with a local backend
.\scripts\test.ps1   # full quality gate: frontend, backend, Rust, launcher scripts
```

GitHub Actions are disabled for this repository. Run `test.ps1` locally before merging. The quality gate runs ESLint, TypeScript, Vitest, Ruff, mypy, pytest, cargo fmt, Clippy, cargo test, and Pester.

Architecture: React/TypeScript UI → thin Tauri/Rust shell → local FastAPI service → OpenCascade exact geometry. See [architecture](docs/architecture.md), [geometry](docs/geometry.md), [DXF export](docs/dxf-export.md), and [UX](docs/ux.md). [CAD2Maxwell_PROJECT_SPEC.md](CAD2Maxwell_PROJECT_SPEC.md) is the product source of truth.

## Contributing and security

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md). Licensed under the [MIT License](LICENSE).
