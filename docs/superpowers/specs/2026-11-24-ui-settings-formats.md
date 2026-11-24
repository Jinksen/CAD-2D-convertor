# UI clarity, settings, formats and documentation

Status: proposal (2026-11-24). Each `T` item is a GitHub issue; the table at the end links them.

The 0.3 workspace works, but it is hard to scan. Findings from the current screenshots
(`docs/images/section-2d.png`, `preview-3d.png`):

| Area | Finding |
|------|---------|
| Ribbon | Flat row of text buttons with no groups. File, plane, compute, export, undo and view controls look alike. XY/XZ/YZ and Compute are repeated in the Section pane. |
| Model Builder | `Preferences` and `Section XY` are tree rows. `Section XY` floats at the bottom, below the groups, far from the bodies it cuts. |
| Body rows | Every row ends with the word `shown`. With 15 bodies that is 15 copies of a word that carries no information. Group members repeat body rows. |
| Right pane | Section settings, section statistics and the group pane are stacked in one column with no clear owner. What the pane shows depends on several selections at once. |
| Messages | Takes about 150 px for two info lines. It cannot be collapsed, filtered or resized. |
| Status bar | Repeats region, error and warning counts from the Section pane. |
| 3D view | The hint text is truncated. No axis triad. The section plane has no label with its offset. |
| 2D view | No scale, no cursor coordinates, no axes. Holes are painted over (known Rune gap). Region names show only after selection. |
| Settings | Only theme and number format exist, and they live in a tree row. Export, viewport and validation options are not configurable. |

Core constraints still apply: exact B-Rep in, analytic curves out; Python owns geometry; the UI
only changes display. No item below may change section results or export geometry unless it
says so and adds regression tests.

## U. Workspace clarity and display

- **T29 Ribbon groups.** Group the ribbon into File (Open STEP, Open project, Save), Section
  (plane XY/XZ/YZ/Custom, Compute), Export (DXF), Edit (Undo, Redo) and View (2D/3D). Each group
  has a small caption. Every button has a tooltip with its shortcut. Plane and Compute exist in
  one place only; the Section pane keeps the offset and custom-plane fields.
- **T30 Model Builder structure.** Order the tree as model file, Bodies, Groups, Sections, like
  the browser in Inventor or the Model Builder in COMSOL. The section node sits directly under the
  model and shows plane and offset (`XY, z = 37.500 mm`). Preferences leave the tree (see T42).
- **T31 Engineering-style tree look.** A small monochrome icon per node type (STEP file, assembly,
  part/body, group, section, plane), vertical indent guide lines, real expand/collapse chevrons,
  one fixed row height, single-line rows with ellipsis and a full-name tooltip. The colour swatch
  stays for bodies. Replace the `shown` text with an eye toggle visible on hover, focus and when
  hidden; hidden and isolated rows are dimmed and keep a text label so status never relies on
  colour or icon alone. Assembly counts stay right-aligned and muted.
- **T32 Tree interactions.** Right-click context menu per node type (Hide/Show, Isolate, Zoom to,
  Add to group, Rename, Properties). Ctrl/Shift multi-select applied to visibility and groups.
  Inline rename with F2 for groups. Drag bodies onto a group to add them. Arrow keys move,
  Left/Right collapse and expand, Enter opens properties.
- **T33 Tree and viewport cross-highlight.** Hovering a tree row highlights the body in 2D and 3D;
  hovering geometry highlights and scrolls to the row. Filter matches are highlighted in the row
  text and the filter shows a match count and clear button. Expand all / collapse all buttons in
  the tree header.
- **T34 Properties pane by selection.** The right pane shows exactly one context with a title
  (`Section XY`, `Body Magnet_1`, `Group North poles`). Section settings show when the section
  node is selected; section statistics move to the section node and the status bar. Long values
  (group members) wrap or list instead of truncating.
- **T35 Messages panel.** Collapsible and resizable, collapsed by default when it holds only info
  messages, opens on the first warning or error. Filter by severity. Clicking the status-bar
  counts opens it. Height is remembered.
- **T36 Resizable side panels.** Drag the splitters between tree, viewport and properties. Panel
  widths and collapsed state persist in preferences. Double-click a splitter to reset.
- **T37 2D viewport aids.** Scale bar, cursor coordinates in the display unit, origin axes,
  optional grid. Hover a region to see its export name. A legend lists visible groups and colours.
- **T38 2D holes.** Holes must stay empty, not painted over. Draw region fills with even-odd
  winding or subtract holes in display tessellation. Display only; export is unchanged. Bug.
- **T39 3D viewport aids.** Axis triad in a corner, section plane labelled with plane and offset,
  hint text moved to a tooltip or the status bar so it never truncates. Clip triangles at the near
  plane instead of dropping them.
- **T40 Measure in 2D.** Distance between two picked points, radius and centre of an arc or circle,
  and exact area, perimeter and centroid of a region in the properties pane. Area and perimeter
  come from the backend (OpenCascade), not from display tessellation.
- **T41 Welcome screen.** Per spec §13.1: Open STEP, Open project, recent projects, the demo
  motor, a link to the documentation, and a drop-a-STEP hint instead of an empty canvas.

## S. Settings

- **T42 Settings dialog.** One dialog (Ctrl+,) with categories, replacing the Preferences tree
  row. Each category has Reset. Stored with `rune::prefs` under `com.cad2maxwell.desktop`.
  - General: theme, recent-projects count, confirm before closing with unsaved changes.
  - Units and numbers: the current number-format options.
  - Viewport: 2D fill opacity, line width, grid on/off, 3D background, preview quality (triangle
    budget), mouse preset (CAD2Maxwell, Inventor, ANSYS).
  - Export: default folder, DXF layer mode and colour mode (T44), ZIP or plain folder.
  - Service: log level, open log folder, restart the geometry service.
  - A settings reference page is added to the docs in the same PR (`docs/settings.md`).
- **T43 Validation tolerances per project.** Expose `TolerancePolicy` (small region, sliver
  ratio, join distance) as an advanced, per-project setting sent with the section request and
  stored in `.c2mproj`. Defaults stay as today. Values outside safe bounds are rejected by the
  backend. A changed tolerance is shown in the section properties and recorded in the manifest.
- **T44 DXF export options.** Layer mode A per component (today), B per group, C hybrid (spec §22);
  colour mode original, group, material or monochrome (spec §23); groups written to the JSON
  manifest with a schema version bump. Layer names stay ASCII-safe and deterministic.
- **T45 Unsaved changes and recent projects.** `*` in the title after a change, a prompt before
  closing or opening another model, and a recent-projects list in the ribbon and welcome screen.
  From `docs/projects-and-formats.md`.

## F. File formats

Import must be exact B-Rep; mesh formats stay rejected. Each new writer reuses the same
validated section result as the DXF and gets deterministic-output tests.

- **T46 IGES import** (`.igs`, `.iges`) through `IGESCAFControl_Reader`, keeping names, colours
  and levels. Recommended next import.
- **T47 OCCT BREP import** (`.brep`). Exact, no names; bodies get deterministic names. Useful for
  fixtures and FreeCAD exports.
- **T48 SVG export.** Arcs map to SVG arcs exactly. 1 user unit = 1 mm, region names as `id`,
  colours from the chosen colour mode. For reports and documentation.
- **T49 Region table (CSV).** Region name, source body, groups, area, perimeter, centroid, hole
  count, computed in the backend from exact geometry. ISO 80000-1 formatting optional, raw mm
  default for tools.
- **T50 FEMM export.** A `.lua` script (or `.fem`) that draws the section in FEMM with arcs as
  arc segments, block labels per region and group names. Gives a free 2D magnetics path that
  needs no ANSYS licence. Adapter, not core.
- **T51 Gmsh `.geo` export.** Points, lines and `Circle` arcs with physical groups per body and
  group. Opens the section to Gmsh, Elmer and GetDP users. Adapter, not core.
- **T52 PyAEDT script export.** A `.py` that builds the regions in Maxwell 2D, names them and
  assigns materials per group. PyAEDT stays optional. Only after the DXF import is verified with
  the Maxwell import checklist.
- **T53 Format spikes.** Evaluate and document, no code unless the result is positive: Motor-CAD
  DXF rules, a 2D STEP export of the section faces (for COMSOL and other CAE that import STEP),
  DXF version option (R12 vs R2018) for older importers, and a PNG/PDF report snapshot.

## X. Other improvements

- **T54 Named sections per project.** Several sections (`Mid stack z = 25 mm`, `End winding
  z = 48 mm`), each with plane, offset and last export path. Switch between them in the tree.
- **T55 Autosave and recovery.** `<name>.c2mproj.autosave` every few minutes, offered on the next
  open when newer than the project.
- **T56 Material labels per group.** Spec §16 and §69: user material labels (copper, electrical
  steel, permanent magnet, air, custom) per group, stored in the project and manifest. Colour can
  suggest a label only through a user rule; it never decides alone.
- **T57 Symmetry and sector extraction.** Spec §67–68: detect rotational symmetry with a
  confidence, and create a 1/N sector only on explicit user action. Lives in a separate
  `machine/` module outside the core geometry engine.
- **T58 Batch CLI.** Several planes or offsets per run from a small JSON job file, writing one
  export per section. For parameter sweeps and CI checks.

## D. Documentation

- **T59 Documentation site.** Build `docs/` with MkDocs Material and publish to GitHub Pages
  from CI. User guide (install, first section, export, Maxwell import, settings, shortcuts,
  troubleshooting), reference (project file schema, manifest schema, formats, tolerances) and
  developer guide (architecture, geometry, Rune app, release). The docs index
  (`docs/README.md`) is the navigation source until then. Screenshots are regenerated when the
  UI items above land.

## Suggested order

1. T29–T35 (the clutter users see first, model tree first), T38 (bug).
2. T42 settings dialog, then T44 export options and T45.
3. T46 IGES, T48 SVG, T49 CSV.
4. T37, T39, T40, T41, T36.
5. The rest, by demand.

## Issues

| Item | Issue |
|------|-------|
| T29 | [#48](https://github.com/Jinksen/CAD-2D-convertor/issues/48) |
| T30 | [#49](https://github.com/Jinksen/CAD-2D-convertor/issues/49) |
| T31 | [#50](https://github.com/Jinksen/CAD-2D-convertor/issues/50) |
| T32 | [#51](https://github.com/Jinksen/CAD-2D-convertor/issues/51) |
| T33 | [#52](https://github.com/Jinksen/CAD-2D-convertor/issues/52) |
| T34 | [#53](https://github.com/Jinksen/CAD-2D-convertor/issues/53) |
| T35 | [#54](https://github.com/Jinksen/CAD-2D-convertor/issues/54) |
| T36 | [#55](https://github.com/Jinksen/CAD-2D-convertor/issues/55) |
| T37 | [#56](https://github.com/Jinksen/CAD-2D-convertor/issues/56) |
| T38 | [#57](https://github.com/Jinksen/CAD-2D-convertor/issues/57) |
| T39 | [#58](https://github.com/Jinksen/CAD-2D-convertor/issues/58) |
| T40 | [#59](https://github.com/Jinksen/CAD-2D-convertor/issues/59) |
| T41 | [#60](https://github.com/Jinksen/CAD-2D-convertor/issues/60) |
| T42 | [#61](https://github.com/Jinksen/CAD-2D-convertor/issues/61) |
| T43 | [#62](https://github.com/Jinksen/CAD-2D-convertor/issues/62) |
| T44 | [#63](https://github.com/Jinksen/CAD-2D-convertor/issues/63) |
| T45 | [#64](https://github.com/Jinksen/CAD-2D-convertor/issues/64) |
| T46 | [#65](https://github.com/Jinksen/CAD-2D-convertor/issues/65) |
| T47 | [#66](https://github.com/Jinksen/CAD-2D-convertor/issues/66) |
| T48 | [#67](https://github.com/Jinksen/CAD-2D-convertor/issues/67) |
| T49 | [#68](https://github.com/Jinksen/CAD-2D-convertor/issues/68) |
| T50 | [#69](https://github.com/Jinksen/CAD-2D-convertor/issues/69) |
| T51 | [#70](https://github.com/Jinksen/CAD-2D-convertor/issues/70) |
| T52 | [#71](https://github.com/Jinksen/CAD-2D-convertor/issues/71) |
| T53 | [#72](https://github.com/Jinksen/CAD-2D-convertor/issues/72) |
| T54 | [#73](https://github.com/Jinksen/CAD-2D-convertor/issues/73) |
| T55 | [#74](https://github.com/Jinksen/CAD-2D-convertor/issues/74) |
| T56 | [#75](https://github.com/Jinksen/CAD-2D-convertor/issues/75) |
| T57 | [#76](https://github.com/Jinksen/CAD-2D-convertor/issues/76) |
| T58 | [#77](https://github.com/Jinksen/CAD-2D-convertor/issues/77) |
| T59 | [#78](https://github.com/Jinksen/CAD-2D-convertor/issues/78) |
