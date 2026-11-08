# Changelog

## 0.2.0

- The desktop app is now a native Rust UI built on the Rune framework. It replaces the React/Vite/Tauri app; no WebView2 runtime is needed.
- Unchanged: the Python/OpenCascade geometry service, the local API, exact B-Rep sectioning and the DXF/manifest export format.
- Installer (Inno Setup, per-user) and portable ZIP both bundle the geometry service. The installer is now built with Inno Setup instead of NSIS.
- 2D section view keeps arcs analytic and tessellates only for display (0.2 px). 3D preview is a CPU rasterizer with component picking.
- Projects (`.c2mproj`) from 0.1.x open unchanged. Saved UI preferences (theme, number format) from the React app are not migrated.
- Known gaps: triangles crossing the 3D near plane are dropped, not clipped; 2D fills skip wires above 3000 points; filled regions with holes are painted over; very large sections (100k curves) exceed the frame budget; builds are not code-signed.

## 0.1.1

Last release of the React/Tauri app.
