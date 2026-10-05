# First local desktop trial

This trial supports local STEP/STP import, exact B-Rep sectioning, component inspection, diagnostics, and analytic DXF export. The compiled UI runs without Vite. Python/OpenCascade runs locally from the existing backend environment.

## Start

Double-click `START-APP.cmd` in the repository folder. The launcher checks port 8000, starts its own backend, authenticates readiness, and opens the compiled desktop app. The status bar should show an online geometry service. Close the app window when finished; the launcher stops its backend. Keep its console open while working.

If the build is missing, run `scripts/build-trial.ps1` after setup. If Python dependencies are missing, run `scripts/setup.ps1`. The local trial is stored at `artifacts/CAD2Maxwell/CAD2Maxwell.exe`; always use the launcher because the executable alone does not start the service. Do not move the executable to another computer: the backend runtime is not bundled yet.

## Import and inspect

1. Click **Open STEP** in the toolbar (or **Choose STEP file**) and select a `.step` or `.stp` file. Alternatively, paste an absolute local path into **STEP path** and click **Import STEP**.
2. Check the body count and import diagnostics. STEP metadata may be missing even when all solids import; inspect the reported diagnostics.
3. Click **3D VIEW** to orbit, zoom, and select a body. The model tree and properties track that selection.
4. Click **2D SECTION**. Choose **XY**, **XZ**, or **YZ** in the toolbar. The offset is a Z, Y, or X coordinate respectively, in millimetres. Initially it is the midpoint of the corresponding model bounds.
5. Click **Compute Section**. Only bodies intersected by that plane produce section geometry. The view fits the resulting curves; use **Fit section**, **+**, **−**, or the mouse wheel to adjust the view. The result reports intersected bodies and closed regions. A section with fewer components than the imported assembly is normal. Holes should stay empty within filled regions.
6. Change the offset as needed. Moving the plane clears the previous result; compute again before export. The 3D view shows the corresponding translucent plane guide.

## Export

When the section passes the implemented checks, **Export draft DXF** becomes available. Errors such as open contours, invalid planar faces, duplicate edges, or positive-area overlaps block export. Tiny edges produce warnings and remain intact.

Click the export button and choose a new `.zip` filename in the Windows Save dialog. The app reports the saved path. Cancelling leaves the section available. Existing files are never overwritten; choose another name if one already exists. Native saving accepts archives up to 64 MiB.

Extract the ZIP into a folder. It contains:

- `section.dxf`: analytic geometry, 1:1 millimetre units, deterministic ASCII-safe component layers.
- `section.json`: source fingerprint, component identities, names, available metadata, plane, and diagnostics.

Import `section.dxf` into Maxwell 2D. Check its dimensions, closed-region formation, holes, and separate object selection against the CAD source. Exports remain marked `draft_unvalidated`: a simple rectangle was verified previously, but complex-assembly Maxwell behavior still requires application testing.

## Troubleshooting and current limits

- **Port already in use:** close the other CAD2Maxwell session. The launcher refuses to attach to or terminate an unrelated process.
- **Startup failed:** inspect `artifacts/logs/*-backend-error.log` and the launcher error. Copy the error when reporting a problem; the session token is not printed in logs.
- **Service restarted:** reimport the CAD and compute again. Exact geometry sessions live in memory.
- **3D preview too large:** the current 50,000-triangle limit affects display only. Exact sectioning remains available.
- **No section:** move the offset inside a body or choose a different principal plane.
- Project save/load, recent files, undo/redo, custom-plane UI, and a self-contained installer are pending. No project state persists between launches.

Source CAD stays local. The launch token changes each time and is required for service requests; unapproved browser origins are rejected. No geometry is repaired automatically.
