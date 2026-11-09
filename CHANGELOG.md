# Changelog

## Unreleased

- Model Builder nests bodies in collapsible folders that follow the STEP assembly structure.
- User groups: create, rename, hide, isolate and delete named sets of bodies. V and I act on the selected group. Groups are display organization only; export is unchanged.
- Projects save groups and collapsed folders as optional fields (0.2 still opens them), store the STEP path relative to the project folder when possible, and are written atomically.
- Installed builds start the geometry service on a free port, so several windows can run at once.
- Messages describe the section plane in the display notation ("XY at z = 37.500 mm") and use sentence-case severity labels.
- The demo motor is split into `Stator_Assembly` and `Rotor_Assembly`; README screenshots show the Rune app.

## 0.2.0

- The desktop app is now a native Rust UI built on the Rune framework. It replaces the React/Vite/Tauri app; no WebView2 runtime is needed.
- Unchanged: the Python/OpenCascade geometry service, the local API, exact B-Rep sectioning and the DXF/manifest export format.
- Installer (Inno Setup, per-user) and portable ZIP both bundle the geometry service. The installer is now built with Inno Setup instead of NSIS.
- 2D section view keeps arcs analytic and tessellates only for display (0.2 px). 3D preview is a CPU rasterizer with component picking.
- Projects (`.c2mproj`) from 0.1.x open unchanged. Saved UI preferences (theme, number format) from the React app are not migrated.
- Known gaps: triangles crossing the 3D near plane are dropped, not clipped; 2D fills skip wires above 3000 points; filled regions with holes are painted over; very large sections (100k curves) exceed the frame budget; builds are not code-signed.

## 0.1.1

Last release of the React/Tauri app.
