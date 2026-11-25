# Changelog

## Unreleased

- 3D preview is anti-aliased in Normal and Fine quality (2× supersampling, edges keep their weight). Draft stays a plain render.

## 0.3.0

Workspace
- Ribbon groups with captions and shortcut tooltips; resizable, collapsible side panels and messages panel (severity filter), layout remembered.
- 2D view: scale bar, cursor readout, origin axes, grid, legend, distance and radius measuring; holes stay empty in filled regions.
- 3D view: axis triad, plane label, hint tooltip, triangles clipped at the near plane.
- Properties pane shows one titled context per tree selection, with exact region area, perimeter, centroid and hole count.

Model tree
- Engineering-style rows (icons, guides, eye toggle), Section node, assembly folders that follow STEP.
- Context menu, multi-select, F2 rename, drag to group, arrow keys, cross-highlight with the viewport, filter count and clear, expand/collapse all.
- User groups with material labels; V and I act on the selected group.

Settings
- Settings dialog (Ctrl+,): General, Units, Viewport, Export, Service. Advanced section tolerances with inline validation.

Import
- IGES (solid B-Reps) and OCCT BREP import next to STEP; drop or open any of them.
- Rotational symmetry hint and explicit 1/N sector section.

Export
- DXF layer modes (component, group, hybrid) and colour modes (original, group, material, monochrome); manifest schema 2.
- CSV region table, SVG, FEMM Lua, Gmsh geo and PyAEDT script writers; batch jobs accept all formats.
- Export uses the Export settings and group material labels.

Projects
- Named sections per project, unsaved-changes marker and prompt, recent projects, welcome screen with demo motor, autosave and recovery offer.
- Groups and collapsed folders are saved as optional fields (0.2 still opens them); STEP path stored relative to the project when possible; atomic writes.
- Installed builds start the geometry service on a free port, so several windows can run.

Docs
- MkDocs Material site with strict build and GitHub Pages deploy; README rewritten; format spikes (Motor-CAD, 2D STEP, DXF R12, PNG/PDF).

Brand
- New section-plane logo in the app, installer and docs site, with a dark variant.

Breaking and behaviour changes
- Export manifest `schema_version` is 2 (new keys `layer_mode`, `color_mode`, `groups`, per-component `layer`, `groups`, `primary_group`, `material_tag`). Default requests produce the same DXF as before.
- Open model clears the current project path; the next save asks for a path.
- Settings moved from the ribbon to the dialog. The old `theme` and `number_format` preferences are still read, so they carry over.
- Messages describe the plane in display notation ("XY at z = 37.500 mm") with sentence-case severity labels.
- The demo motor is split into `Stator_Assembly` and `Rotor_Assembly`.

## 0.2.0

- The desktop app is now a native Rust UI built on the Rune framework. It replaces the React/Vite/Tauri app; no WebView2 runtime is needed.
- Unchanged: the Python/OpenCascade geometry service, the local API, exact B-Rep sectioning and the DXF/manifest export format.
- Installer (Inno Setup, per-user) and portable ZIP both bundle the geometry service. The installer is now built with Inno Setup instead of NSIS.
- 2D section view keeps arcs analytic and tessellates only for display (0.2 px). 3D preview is a CPU rasterizer with component picking.
- Projects (`.c2mproj`) from 0.1.x open unchanged. Saved UI preferences (theme, number format) from the React app are not migrated.
- Known gaps: triangles crossing the 3D near plane are dropped, not clipped; 2D fills skip wires above 3000 points; filled regions with holes are painted over; very large sections (100k curves) exceed the frame budget; builds are not code-signed.

## 0.1.1

Last release of the React/Tauri app.
