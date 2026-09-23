# CAD2Maxwell — Codex Implementation Specification

## 0. Purpose of this document

This document is the authoritative implementation brief for **CAD2Maxwell**, a Windows desktop engineering application that converts 3D CAD assemblies — initially STEP/STP — into clean, validated, metadata-preserving 2D cross-sections suitable for **ANSYS Maxwell 2D**.

The application is intended primarily for electrical-machine geometry, but the core architecture must remain generic enough to support other electromagnetic 2D preprocessing tasks.

Codex should treat this document as the primary product and architecture specification. Do not replace the architecture with a simpler monolithic prototype unless an explicit task asks for it.

---

# 1. Product vision

CAD2Maxwell sits between mechanical CAD and electromagnetic simulation.

Primary workflow:

```text
Autodesk Inventor / STEP assembly
        ↓
CAD2Maxwell
        ↓
3D model inspection
        ↓
Select / detect section plane
        ↓
Exact B-Rep cross-section
        ↓
Preserve component names, colors and hierarchy
        ↓
Validate 2D geometry
        ↓
Preview and edit mapping
        ↓
DXF + manifest export
        ↓
ANSYS Maxwell 2D
```

Long-term workflow:

```text
Inventor
   ↓
CAD2Maxwell
   ↓
Exact section geometry
   ↓
Material / winding / group mapping
   ↓
PyAEDT
   ↓
ANSYS Maxwell 2D project generated automatically
```

The main product value is NOT merely "convert STEP to DXF".

The product value is:

> Convert engineering CAD assemblies into clean, traceable, simulation-ready 2D electromagnetic geometry while preserving component identity and minimizing manual repair in Maxwell.

---

# 2. Primary user

Primary target user:

- electrical-machine engineer,
- university researcher,
- motor / generator designer,
- electromagnetic simulation engineer,
- user working with Autodesk Inventor and ANSYS Maxwell.

Typical models contain:

- stator laminations,
- rotor laminations,
- shafts,
- permanent magnets,
- copper/aluminum windings,
- air regions,
- insulation,
- repeated slots,
- repeated coils,
- assemblies with many occurrences.

The application must feel like an engineering tool, not a generic CAD viewer.

---

# 3. Core user problem

ANSYS Maxwell 2D expects planar closed regions.

Mechanical CAD systems primarily store 3D B-Rep solids.

Direct STEP import into Maxwell 2D often causes one or more of the following:

- 3D objects are not converted into the required 2D planar regions,
- individual components become difficult to select,
- material assignment becomes cumbersome,
- names and colors are lost,
- overlapping or duplicated edges appear,
- contours may not be closed,
- section geometry is not obvious,
- assemblies require extensive manual reconstruction.

CAD2Maxwell must solve the intermediate preprocessing step explicitly.

---

# 4. MVP scope

Version 0.1 must support:

1. STEP/STP import.
2. Multiple solids / assembly-like STEP content.
3. Per-body metadata:
   - original name when available,
   - safe exported name,
   - original color when available,
   - body index / stable internal ID.
4. Interactive 3D preview.
5. Section plane:
   - XY,
   - YZ,
   - XZ,
   - custom plane,
   - offset along normal.
6. Exact B-Rep intersection with the plane.
7. Conversion of intersection geometry into 2D local coordinates.
8. 2D preview.
9. Per-region selection.
10. Validation:
    - closed/open contour,
    - duplicate edges,
    - self-intersections,
    - overlapping regions,
    - tiny edges,
    - non-manifold / ambiguous contour topology where detectable.
11. DXF export:
    - 1:1 scale,
    - units explicitly stored,
    - separate layer per exported object or logical group,
    - original/safe names,
    - colors preserved where possible,
    - exact LINE / ARC / CIRCLE when possible.
12. JSON manifest export with metadata.
13. Project save/load using a native CAD2Maxwell project file.
14. Undo/redo for UI-side mapping changes.
15. Recent-files list.
16. Dark and light themes.
17. Error log suitable for engineering debugging.

Version 0.1 must NOT attempt full Maxwell simulation setup.

---

# 5. Long-term scope

Future versions may add:

- Autodesk Inventor direct integration,
- Inventor Add-In,
- direct Maxwell project generation,
- PyAEDT integration,
- automatic material mapping,
- automatic winding grouping,
- symmetry / periodic sector detection,
- air-gap detection,
- rotor/stator identification,
- automatic machine center detection,
- slot/pole counting,
- region generation,
- automatic mesh hints,
- Motor-CAD interoperability,
- batch processing,
- CLI mode,
- scripting API,
- reusable templates.

Architecture must not prevent these features.

---

# 6. Technology stack

## 6.1 Desktop shell and GUI

Use:

- **Tauri**
- **React**
- **TypeScript**
- **Vite**

Reason:

- polished modern desktop UI,
- smaller footprint than Electron,
- native desktop packaging,
- easy implementation of docking-like engineering panels,
- good long-term UI maintainability,
- strong separation between GUI and geometry engine.

Do NOT implement the geometry engine in TypeScript.

---

## 6.2 UI component stack

Recommended:

- React
- TypeScript strict mode
- Tailwind CSS
- shadcn/ui or equivalent accessible primitives
- Lucide icons
- Zustand for lightweight UI state
- TanStack Query only if asynchronous service state benefits from it
- React Hook Form + Zod for editable property panels
- React Router only if truly needed; this should primarily be a desktop workspace, not a website

Avoid excessive dependency count.

---

## 6.3 3D rendering

Preferred implementation:

- Three.js via React Three Fiber

The 3D renderer is for:
- visualization,
- selection,
- section plane manipulation,
- body highlighting,
- orientation helpers.

The 3D viewer must NOT be treated as the source of geometric truth.

Geometric truth remains in the Python/OpenCascade backend.

For imported STEP geometry, backend should provide render meshes separately from exact B-Rep geometry.

---

## 6.4 2D rendering

Use a dedicated 2D engineering viewport.

Recommended options:

- SVG for initial MVP if performance is sufficient,
- Canvas/WebGL later for larger models.

Requirements:

- pan,
- zoom,
- zoom-to-fit,
- region hover,
- region click selection,
- layer visibility,
- color display,
- center/origin display,
- coordinate readout,
- grid toggle,
- contour diagnostic overlays.

Do not use raster screenshots as the 2D representation.

---

## 6.5 Geometry backend

Use:

- Python 3.12+
- OpenCascade bindings, preferably OCP / CadQuery ecosystem where appropriate
- NumPy
- Shapely only for supplementary 2D checks, never as the only geometric kernel
- ezdxf for DXF creation
- Pydantic for backend schemas
- pytest for testing

The backend owns:

- STEP import,
- B-Rep topology,
- section operations,
- curve extraction,
- geometry validation,
- exact geometric metadata,
- DXF export,
- manifest generation.

---

## 6.6 Backend process model

Do NOT run heavy OpenCascade operations directly in the Tauri UI thread.

Preferred architecture:

```text
Tauri desktop
    │
    ├── React UI
    │
    └── Rust command bridge
            │
            ▼
      Python geometry service
```

For MVP, the Python service may run as a local child process using a strict request/response protocol.

Preferred communication:

- localhost HTTP with FastAPI, OR
- JSON-RPC over stdin/stdout.

Choose one and document it.

Recommendation for MVP:
- FastAPI bound to `127.0.0.1` on a dynamically allocated port,
- Tauri starts and supervises the Python process,
- backend never exposes itself beyond localhost,
- token or random session secret for local requests.

Do not introduce Docker.

---

# 7. Naming rules

The app must distinguish between:

- `display_name`
- `source_name`
- `export_name`

Example:

```text
source_name  = "Cívka"
display_name = "Cívka 13"
export_name  = "Civka_13"
```

Export names must be safe for ANSYS scripting.

Default allowed characters:

```text
A-Z
a-z
0-9
_
-
```

Provide deterministic transliteration from Czech and other Latin characters.

Avoid spaces by default in export names.

Duplicate names must be resolved deterministically:

```text
Civka
Civka_02
Civka_03
```

Never use unstable random names in exported geometry.

---

# 8. Metadata preservation

For every imported solid/body, attempt to preserve:

- source name,
- occurrence name if available,
- source color,
- source hierarchy,
- body index,
- source STEP entity reference if possible,
- source material name if encoded,
- assembly path if recoverable.

Not all STEP files contain all metadata.

The UI must distinguish:

- data present in source,
- data inferred by CAD2Maxwell,
- user-edited data.

Never present inferred information as if it were explicitly stored in STEP.

---

# 9. Internal data model

Use stable typed schemas.

Example conceptual model:

```python
Component:
    id: UUID
    source_id: str | None
    source_name: str | None
    display_name: str
    export_name: str
    source_color: RGB | None
    user_color: RGB | None
    hierarchy_path: list[str]
    visible: bool
    export_enabled: bool
    group_id: UUID | None
    material_id: UUID | None
    exact_body_ref: backend-only reference
```

Section result:

```python
SectionRegion:
    id: UUID
    component_id: UUID
    outer_wire: Wire2D
    inner_wires: list[Wire2D]
    area_mm2: float
    centroid_mm: [float, float]
    bbox_mm: [xmin, ymin, xmax, ymax]
    is_closed: bool
    validation_status: enum
```

Curve primitives:

```text
Line2D
Arc2D
Circle2D
Ellipse2D
BSpline2D
```

Preserve exact analytic curves whenever possible.

Do not flatten everything into polylines unless explicitly requested.

---

# 10. CAD2Maxwell project file

Use a human-readable project format.

Recommended extension:

```text
.c2mproj
```

Internally JSON for MVP.

Example:

```json
{
  "schema_version": 1,
  "source": {
    "path": "C:/Projects/Motor/Sestava-plechu.stp",
    "sha256": "..."
  },
  "units": "mm",
  "section_plane": {
    "origin": [0.0, 0.0, 0.0],
    "normal": [0.0, 0.0, 1.0],
    "x_axis": [1.0, 0.0, 0.0]
  },
  "components": [],
  "export_settings": {}
}
```

Never serialize opaque OpenCascade objects directly into the project file.

Rebuild geometry from source CAD on project reopen.

Warn if source file hash changed.

---

# 11. Main GUI concept

The UI should look like a professional CAE/CAD preprocessor.

Target layout:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ File  Edit  View  Section  Validate  Export  Tools  Help                  │
├─────────────────────────────────────────────────────────────────────────────┤
│ Toolbar: Open | Save | Plane | Auto Section | Validate | Export Maxwell   │
├──────────────────┬──────────────────────────────────────┬───────────────────┤
│ MODEL TREE       │                                      │ PROPERTIES        │
│                  │             VIEWPORT                 │                   │
│ ▾ Assembly       │                                      │ Name              │
│   ◉ Stator       │   [3D / 2D tabs or split mode]      │ Material          │
│   ◉ Rotor        │                                      │ Color             │
│   ▾ Windings     │                                      │ Export Name       │
│     ◉ Coil 01    │                                      │ Group             │
│     ◉ Coil 02    │                                      │                   │
│                  │                                      │                   │
├──────────────────┼──────────────────────────────────────┼───────────────────┤
│ LAYERS/GROUPS    │ DIAGNOSTICS / SECTION RESULT         │                   │
│                  │                                      │                   │
├──────────────────┴──────────────────────────────────────┴───────────────────┤
│ ✓ 34 bodies | ✓ 34 regions | 0 overlaps | 0 open loops | units: mm       │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

# 12. GUI design principles

The GUI must be cleaner than a typical engineering prototype.

Principles:

1. Dense but readable.
2. No oversized consumer-app cards.
3. Engineering information hierarchy.
4. Keyboard-friendly.
5. Dock/panel paradigm.
6. Persistent selection state.
7. Context-sensitive properties.
8. Clear distinction between:
   - source CAD,
   - derived section,
   - exported Maxwell representation.
9. Avoid modal dialogs where a side panel or inline control is better.
10. Destructive actions require confirmation.
11. Long operations display progress and remain cancellable where feasible.
12. Errors should be actionable.

---

# 13. Main screens / states

## 13.1 Welcome screen

Show:

- New project
- Open STEP/STP
- Open CAD2Maxwell project
- Recent projects
- Example project
- Documentation link

Do not show a blank engineering canvas with no guidance.

---

## 13.2 Import screen

After selecting STEP:

Show progress stages:

```text
Reading STEP...
Building topology...
Extracting metadata...
Generating preview mesh...
Detecting candidate section planes...
```

Then show import summary:

```text
34 solids
34 named bodies
5 unique source names
6 source colors
Units: mm
Bounding box: ...
```

---

## 13.3 Section workspace

Must allow:

- XY / XZ / YZ buttons,
- custom plane,
- click planar face to align plane,
- offset slider + numeric input,
- flip normal,
- reset,
- section preview update.

Expensive geometry recomputation should be debounced.

Do not recompute exact section on every pixel of slider movement.
Use:
- lightweight visual preview while dragging,
- exact recompute on release or after debounce.

---

## 13.4 3D viewport controls

Required:

- orbit,
- pan,
- zoom,
- fit all,
- standard views,
- perspective/orthographic toggle,
- section plane visualization,
- selected component highlight,
- isolate,
- hide/show,
- transparency override,
- coordinate triad.

---

## 13.5 2D section viewport

Required:

- pan,
- zoom,
- fit,
- select region,
- select component by region,
- display original colors,
- display export colors,
- display contour status,
- display open-end markers,
- overlap heat/overlay,
- region IDs toggle,
- origin marker,
- grid toggle.

---

# 14. Component tree

Tree should support:

- search,
- multi-selection,
- hide/show,
- isolate,
- color indicator,
- validation indicator,
- export checkbox,
- group assignment.

Example:

```text
▾ Assembly
  ● Stator                     ✓
  ● Rotor                      ✓
  ▾ Windings (24)
    ● Cívka 01                 ✓
    ● Cívka 02                 ✓
    ● Cívka 03                 ✓
  ▾ field-1 (4)
  ▾ field-2 (4)
```

Support grouping repeated names automatically.

---

# 15. Properties panel

When component selected, show:

```text
Source
------
Source Name
Occurrence / Body ID
Source Color
Source Hierarchy
Source Material

CAD2Maxwell
-----------
Display Name
Export Name
Group
Material Mapping
Export Enabled
Color Override

Section
-------
Regions Produced
Area
Centroid
Contour Count
Validation Status
```

Changes should not modify original STEP.

---

# 16. Material mapping

MVP should support user-defined material labels even if direct Maxwell import is not yet implemented.

Built-in common labels may include:

- air,
- copper,
- aluminum,
- electrical steel,
- structural steel,
- permanent magnet,
- insulation,
- custom.

Do not pretend these labels are exact Maxwell material library entries unless confirmed.

Store mapping in manifest.

---

# 17. Section algorithm

The exact section pipeline should be conceptually:

```text
STEP Body
   ↓
OpenCascade B-Rep solid
   ↓
Section plane
   ↓
BRepAlgoAPI_Section or equivalent
   ↓
3D intersection edges
   ↓
Connect edges into wires
   ↓
Transform into plane-local 2D coordinates
   ↓
Classify outer / inner loops
   ↓
Create planar regions
   ↓
Validate
```

Requirements:

- use a tolerance policy,
- never silently bridge large gaps,
- log every repair,
- preserve source component ownership,
- handle multiple disconnected section regions from one body,
- handle holes,
- handle tangent contacts gracefully,
- detect degenerate intersections.

---

# 18. Tolerance policy

Centralize geometric tolerances.

Example configurable defaults:

```text
linear_tolerance_mm      = 1e-6
join_tolerance_mm        = 1e-5
tiny_edge_threshold_mm   = 1e-4
area_tolerance_mm2       = 1e-8
overlap_tolerance_mm2    = 1e-8
```

Actual defaults may be adjusted after testing.

Never scatter hard-coded tolerances throughout the codebase.

Create one configuration object.

---

# 19. Geometry repair philosophy

Automatic repair must be conservative.

Allowed automatic repairs:

- merge duplicate coincident edges,
- join endpoints inside a very small tolerance,
- remove zero-length or numerically degenerate entities.

Do NOT automatically:

- close large gaps,
- delete significant features,
- simplify arcs to lines,
- union separate components,
- subtract overlaps without user confirmation.

Every repair must be recorded in diagnostics.

---

# 20. Validation system

Each region receives:

```text
PASS
WARNING
ERROR
```

Validation checks:

### Topology
- open wire,
- disconnected wire,
- self-intersection,
- invalid hole nesting,
- duplicate contour.

### Geometry
- tiny edge,
- tiny area,
- zero area,
- duplicate edge,
- unsupported spline,
- extreme coordinate magnitude.

### Cross-component
- overlap,
- coincident boundaries,
- contained region ambiguity.

### Export
- duplicate export name,
- invalid characters,
- unsupported layer name,
- missing units.

---

# 21. Diagnostics panel

Provide engineering-style diagnostics:

```text
[ERROR] Coil_07: contour is open by 0.034 mm
[WARN ] Rotor: 2 edges below tiny-edge threshold
[WARN ] Magnet_03 overlaps Rotor by 0.0021 mm²
[INFO ] Stator: 48 analytic arcs preserved
```

Clicking a diagnostic must select and zoom to the affected geometry.

---

# 22. DXF export

Use ezdxf.

Requirements:

- DXF version chosen for compatibility,
- units stored as mm,
- 1:1 coordinates,
- center/origin option,
- separate layers,
- source colors where possible,
- exact arc/circle geometry,
- no arbitrary rescaling.

Export modes:

### Mode A — Per component
```text
Stator
Rotor
Civka_01
Civka_02
...
```

### Mode B — Per logical group
```text
Stator
Rotor
Windings
Magnets
Air
```

### Mode C — Hybrid
Each component remains identifiable while sharing logical layer/group metadata.

For MVP, Mode A is mandatory.

---

# 23. DXF color behavior

Preserve source component color where available.

Use:
- DXF true color when supported,
- nearest ACI fallback if needed.

User may switch export appearance:

```text
Original CAD colors
Material colors
Group colors
Monochrome
```

Default:
`Original CAD colors`.

---

# 24. Export manifest

Always export a sidecar JSON unless user disables it.

Example:

```json
{
  "schema_version": 1,
  "source_file": "Sestava-plechu.stp",
  "units": "mm",
  "section_plane": {
    "origin": [0, 0, 0],
    "normal": [0, 0, 1]
  },
  "components": [
    {
      "id": "cmp-001",
      "source_name": "Cívka",
      "display_name": "Cívka 01",
      "export_name": "Civka_01",
      "source_color": [188, 80, 47],
      "material_tag": "copper",
      "regions": ["reg-001"]
    }
  ]
}
```

This manifest is the bridge for future PyAEDT automation.

---

# 25. Maxwell compatibility strategy

MVP goal:

Produce DXF geometry that imports cleanly into ANSYS Maxwell 2D.

Recommended import assumptions:

- millimeters,
- no diacritics in exported object/layer names,
- no spaces by default,
- closed planar contours,
- unique layers,
- analytic primitives where possible.

Future PyAEDT integration should:

1. create/open AEDT project,
2. create Maxwell 2D design,
3. import geometry,
4. map regions to objects,
5. assign names,
6. assign materials,
7. optionally create region/boundary settings.

Do not implement these automatically until geometry export is reliable.

---

# 26. Error handling

Backend errors must be structured.

Example:

```json
{
  "code": "SECTION_OPEN_WIRE",
  "message": "Section produced an open wire.",
  "component_id": "cmp-012",
  "details": {
    "gap_mm": 0.034
  }
}
```

UI must never show a raw Python traceback as the primary user-facing error.

Traceback may be accessible in:
`View → Developer Log`.

---

# 27. Logging

Use structured logs.

Levels:

- debug
- info
- warning
- error

Log:

- source file hash,
- import duration,
- body count,
- section plane,
- section duration,
- validation summary,
- export summary,
- backend version,
- geometry kernel version.

Never log full personal file paths in telemetry because there should be no telemetry by default.

---

# 28. Privacy and networking

The app should be local-first.

Default:

- no cloud upload,
- no account requirement,
- no analytics,
- no external processing.

All CAD processing remains on the user's machine.

If update checking is later added, make it transparent and optional.

---

# 29. Performance targets

Initial targets for a typical machine model:

```text
STEP size:           < 200 MB
Bodies:              < 1,000
Interactive preview: responsive
Section operation:   target < 5 s for normal assemblies
UI blocking:         none during long operation
```

These are targets, not hard guarantees.

Heavy operations must run asynchronously.

---

# 30. Caching

Cache expensive derived data using source file hash + relevant parameters.

Cache candidates:

- preview meshes,
- body metadata,
- section results,
- validation results.

Cache invalidation must be deterministic.

Changing:
- section plane,
- tolerance settings,
- source file

must invalidate relevant geometry cache.

Changing:
- display color,
- material label

should NOT force section recomputation.

---

# 31. Repo architecture

Recommended monorepo:

```text
cad2maxwell/
│
├── README.md
├── AGENTS.md
├── CLAUDE.md
├── LICENSE
├── .gitignore
├── .editorconfig
│
├── apps/
│   └── desktop/
│       ├── src/                    # React/TypeScript
│       ├── src-tauri/              # Tauri/Rust shell
│       ├── package.json
│       └── vite.config.ts
│
├── backend/
│   ├── pyproject.toml
│   ├── src/
│   │   └── cad2maxwell/
│   │       ├── api/
│   │       ├── cad/
│   │       ├── geometry/
│   │       ├── validation/
│   │       ├── export/
│   │       ├── project/
│   │       ├── models/
│   │       └── util/
│   └── tests/
│
├── shared/
│   ├── schemas/
│   └── fixtures/
│
├── docs/
│   ├── architecture.md
│   ├── geometry.md
│   ├── dxf-export.md
│   └── ux.md
│
└── scripts/
    ├── dev.ps1
    ├── test.ps1
    └── package.ps1
```

---

# 32. Backend package architecture

```text
cad2maxwell/
├── api/
│   ├── app.py
│   ├── routes_import.py
│   ├── routes_section.py
│   ├── routes_validation.py
│   └── routes_export.py
│
├── cad/
│   ├── step_reader.py
│   ├── metadata.py
│   ├── colors.py
│   ├── hierarchy.py
│   └── meshing.py
│
├── geometry/
│   ├── plane.py
│   ├── section.py
│   ├── transforms.py
│   ├── wires.py
│   ├── curves.py
│   ├── classification.py
│   └── tolerance.py
│
├── validation/
│   ├── topology.py
│   ├── overlaps.py
│   ├── duplicates.py
│   ├── tiny_features.py
│   └── report.py
│
├── export/
│   ├── dxf_writer.py
│   ├── manifest.py
│   ├── naming.py
│   └── colors.py
│
├── project/
│   ├── schema.py
│   ├── loader.py
│   └── saver.py
│
├── models/
│   ├── component.py
│   ├── section.py
│   ├── validation.py
│   └── export.py
│
└── util/
    ├── hashing.py
    ├── logging.py
    └── ids.py
```

---

# 33. Frontend architecture

```text
src/
├── app/
│   ├── App.tsx
│   ├── routes.ts
│   └── providers.tsx
│
├── features/
│   ├── project/
│   ├── import/
│   ├── model-tree/
│   ├── viewport-3d/
│   ├── viewport-2d/
│   ├── section-plane/
│   ├── properties/
│   ├── validation/
│   └── export/
│
├── components/
│   ├── ui/
│   └── engineering/
│
├── state/
│   ├── projectStore.ts
│   ├── selectionStore.ts
│   └── uiStore.ts
│
├── api/
│   ├── client.ts
│   └── types.ts
│
├── lib/
│   ├── units.ts
│   ├── formatting.ts
│   └── geometry-display.ts
│
└── styles/
```

Feature folders should own feature-specific components.

Avoid a giant global `components` directory.

---

# 34. File size and complexity rules

Target:

- most source files < 300 lines,
- hard preference < 500 lines,
- functions preferably < 50 lines,
- React components should remain focused,
- geometry algorithms get dedicated modules,
- no "utils.py" dumping ground.

If a file exceeds ~500 lines, Codex should consider splitting it.

Do not split mechanically when cohesion would become worse.

---

# 35. Rust responsibility

Rust in Tauri should remain a thin desktop layer.

Responsibilities:

- window lifecycle,
- native file dialogs,
- process supervision,
- secure local backend startup,
- app packaging,
- native commands,
- filesystem permissions.

Do NOT reimplement OpenCascade geometry in Rust for MVP.

---

# 36. Python responsibility

Python is the geometry authority.

Responsibilities:

- CAD import,
- exact geometry,
- metadata,
- validation,
- DXF export,
- future PyAEDT integration.

Do not leak OpenCascade objects into the frontend protocol.

Return serializable DTOs only.

---

# 37. API design

Use versioned local API:

```text
/api/v1/health
/api/v1/import
/api/v1/model/{project_id}
/api/v1/section
/api/v1/section/{id}
/api/v1/validate
/api/v1/export/dxf
```

Long-running calls should support job IDs:

```text
POST /section
→ job_id

GET /jobs/{job_id}
→ progress
```

Later:
WebSocket/SSE for progress if beneficial.

---

# 38. Preview mesh protocol

Backend should generate a visualization mesh per body.

Return:

- positions,
- normals,
- indices,
- component ID,
- color.

For large meshes:
- use binary transport/file cache rather than huge JSON arrays.

MVP may use temporary `.glb` or `.gltf` files generated locally.

Preferred:
- one GLB with per-body node IDs or separate body metadata mapping.

Exact geometry remains backend-only.

---

# 39. Section result transport

2D section DTO should contain analytic primitives.

Example:

```json
{
  "region_id": "reg-001",
  "component_id": "cmp-001",
  "loops": [
    {
      "role": "outer",
      "curves": [
        {
          "type": "line",
          "p1": [1.0, 2.0],
          "p2": [3.0, 2.0]
        },
        {
          "type": "arc",
          "center": [3.0, 4.0],
          "radius": 2.0,
          "start_angle": -90.0,
          "end_angle": 0.0
        }
      ]
    }
  ]
}
```

---

# 40. Selection model

Selection must synchronize between:

- model tree,
- 3D viewport,
- 2D viewport,
- diagnostics,
- properties panel.

Single selection:
- click.

Multi-selection:
- Ctrl+click.

Range where meaningful:
- Shift.

Esc clears selection.

Double-click:
- frame/zoom to selected object.

---

# 41. Keyboard shortcuts

Recommended:

```text
Ctrl+O        Open
Ctrl+S        Save project
Ctrl+Shift+S  Save as
Ctrl+Z        Undo
Ctrl+Y        Redo
F             Fit selection
Shift+F       Fit all
Delete        no destructive CAD deletion in MVP
V             Toggle visibility
I             Isolate
1/2/3         XY/XZ/YZ standard planes
Ctrl+E        Export
Ctrl+L        Validation panel
```

Avoid conflicting shortcuts.

---

# 42. Undo/redo

Undo/redo should cover user-level project edits such as:

- rename,
- group assignment,
- material label,
- visibility,
- export enabled,
- section plane changes.

Do not attempt to undo low-level imported CAD topology.

Use command/action history in frontend project state.

---

# 43. Themes

Support:

- system,
- dark,
- light.

Default:
system.

Engineering canvas background should remain configurable because geometry visibility varies with source colors.

---

# 44. Accessibility

Minimum:

- keyboard navigable controls,
- clear focus state,
- tooltips,
- sufficient contrast,
- icons paired with accessible labels,
- do not communicate validation status by color alone.

Use:
✓ PASS
⚠ WARNING
✕ ERROR

plus color.

---

# 45. Testing strategy

## Backend unit tests

Test:

- naming sanitizer,
- coordinate transforms,
- curve conversion,
- wire classification,
- tolerance logic,
- manifest schema,
- DXF layer generation.

## Geometry regression tests

Create fixture STEP models:

1. single box,
2. concentric cylinders,
3. body with hole,
4. multiple bodies,
5. tangent bodies,
6. tiny gap,
7. repeated coil-like bodies,
8. electrical-machine fixture.

For each fixture store expected:

- body count,
- section region count,
- bounding box,
- total area within tolerance,
- number of holes,
- validation status.

## Frontend tests

Use:
- Vitest,
- React Testing Library.

Test:
- selection sync,
- tree filtering,
- properties editing,
- validation rendering,
- export settings.

## E2E

Use Playwright if Tauri integration is practical; otherwise use web UI test harness initially.

---

# 46. Primary regression fixture

Use the provided real engineering STEP model as a regression fixture only if licensing/privacy allows it in the local development repository.

Known expected behavior from current conversion workflow:

- 34 solids/bodies,
- separate regions should remain individually traceable,
- geometry must preserve exact relative placement,
- section should support original source names/colors when STEP metadata contains them,
- DXF must import into Maxwell 2D at 1:1 scale.

Do not hard-code these values into production logic.

Use them only as a regression test expectation.

---

# 47. Acceptance criteria for v0.1

Version 0.1 is acceptable when all of the following are true:

1. User can open a STEP file.
2. App lists all imported solids.
3. User can inspect the model in 3D.
4. User can choose a section plane.
5. App computes an exact section.
6. 2D viewport displays resulting regions.
7. Regions remain linked to source components.
8. Source names are displayed when available.
9. Source colors are displayed when available.
10. Export-safe names are generated.
11. Validation reports open contours and overlaps.
12. User can export DXF in mm at 1:1.
13. DXF opens in ANSYS Maxwell 2D.
14. Exported regions can be converted/used as separate selectable Maxwell objects.
15. Sidecar manifest maps source component → exported object.
16. App does not freeze during normal section calculations.
17. Project can be saved and reopened.
18. No CAD data leaves the machine by default.

---

# 48. Milestone plan

## Milestone 0 — Repository foundation

Deliver:

- monorepo,
- React/Tauri shell,
- Python backend,
- dev scripts,
- linting,
- formatting,
- CI,
- AGENTS.md,
- basic README.

No geometry yet.

---

## Milestone 1 — STEP import

Deliver:

- backend STEP import,
- body enumeration,
- stable body IDs,
- metadata extraction,
- source color extraction,
- bounding box,
- preview mesh,
- import summary API.

UI:

- open file,
- progress,
- model tree,
- 3D model display.

---

## Milestone 2 — Section engine

Deliver:

- Plane model,
- XY/XZ/YZ,
- custom plane,
- exact section,
- local 2D transform,
- wire assembly,
- region classification.

UI:

- section plane controls,
- 2D preview.

---

## Milestone 3 — Validation

Deliver:

- open contours,
- duplicate edges,
- overlaps,
- tiny features,
- diagnostics model.

UI:

- validation panel,
- click diagnostic → focus geometry.

---

## Milestone 4 — DXF export

Deliver:

- 1:1 mm export,
- one layer per component,
- names,
- colors,
- LINE/ARC/CIRCLE,
- manifest JSON.

Create Maxwell import verification checklist.

---

## Milestone 5 — Project workflow

Deliver:

- `.c2mproj`,
- recent projects,
- save/load,
- settings,
- undo/redo,
- export presets.

---

## Milestone 6 — UX refinement

Deliver:

- polished panel layout,
- keyboard shortcuts,
- themes,
- loading/progress states,
- empty states,
- diagnostic overlays,
- performance tuning.

---

# 49. Future milestone — Inventor integration

Two possible approaches.

## Option A — External automation

Python + COM automation using pywin32.

Advantages:
- fast to prototype,
- can query active document,
- can read iProperties/occurrences.

Disadvantages:
- weaker UX than native add-in.

## Option B — Native Inventor Add-In

C# + Visual Studio.

Add ribbon button:

```text
CAD2Maxwell
  ├── Send Assembly
  ├── Use Selected Face as Section Plane
  └── Export Maxwell 2D
```

Recommended long-term direction.

The desktop app should remain usable without Inventor.

---

# 50. Future milestone — PyAEDT integration

Do this only after DXF export is robust.

Potential workflow:

```text
CAD2Maxwell project
       ↓
Generate Maxwell package
       ↓
PyAEDT launches AEDT
       ↓
Create Maxwell 2D design
       ↓
Import geometry
       ↓
Rename objects
       ↓
Assign materials
       ↓
Save AEDT project
```

Provide dry-run mode before applying modifications.

---

# 51. Security

Local backend requirements:

- bind only to 127.0.0.1,
- random session token,
- reject external origins,
- validate file paths,
- never execute CAD-embedded scripts,
- never evaluate arbitrary Python from imported project files.

Tauri filesystem permissions should follow least privilege.

---

# 52. Dependency policy

Before adding dependency:

1. confirm active maintenance,
2. confirm compatible license,
3. confirm it meaningfully reduces complexity,
4. avoid overlapping libraries for the same task.

Prefer:
- stable,
- boring,
- documented libraries.

Avoid framework churn.

---

# 53. Code quality

Python:

- Ruff
- Black or Ruff formatter
- mypy or pyright
- pytest
- Pydantic models

TypeScript:

- strict
- ESLint
- Prettier
- Vitest
- no `any` without explicit reason

Rust:

- rustfmt
- clippy

CI must run:

```text
frontend lint
frontend typecheck
frontend tests
backend lint
backend typecheck
backend tests
rust fmt check
rust clippy
```

---

# 54. Documentation requirements

Maintain:

```text
README.md
docs/architecture.md
docs/geometry.md
docs/dxf-export.md
docs/ux.md
```

For geometry algorithms, explain WHY.

Do not rely only on code comments.

---

# 55. README minimum

README should include:

- what CAD2Maxwell is,
- screenshot placeholder,
- current status,
- supported file types,
- developer prerequisites,
- setup,
- run development,
- test,
- package,
- known limitations,
- roadmap.

---

# 56. Versioning

Use semantic versioning.

Early:

```text
0.1.0
0.2.0
...
```

Project schema has independent schema version.

DXF manifest also has schema version.

---

# 57. Windows developer environment

Recommended:

- Windows 11
- Git
- Python 3.12
- uv for Python dependency management
- Node.js current LTS
- pnpm
- Rust stable
- Visual Studio Build Tools / required MSVC toolchain
- PyCharm for Python geometry work
- VS Code or WebStorm for React/Tauri
- Visual Studio later for Inventor C# Add-In

If the user wants one IDE, JetBrains Rider + PyCharm can work well, but separate specialized IDEs are acceptable.

---

# 58. Development commands

Provide PowerShell scripts.

Desired developer experience:

```powershell
.\scripts\dev.ps1
.\scripts\test.ps1
.\scripts\package.ps1
```

`dev.ps1` should:

1. create/check Python environment,
2. start geometry backend,
3. start Tauri dev app.

Avoid requiring the developer to manually open three terminals.

---

# 59. Packaging

Goal:

Single Windows installer.

Bundle:

- Tauri app,
- Python runtime/service or packaged Python backend,
- required geometry libraries.

Potential backend packaging:
- PyInstaller,
- Nuitka,
- or embedded Python distribution.

Choose based on OpenCascade compatibility tests.

Do not decide solely on bundle size.

---

# 60. UI visual direction

Use a restrained engineering aesthetic.

Recommended:

- dark graphite or neutral gray work area,
- subtle panel separators,
- one accent color,
- compact controls,
- tabular numeric fields,
- monospace only for IDs/logs/code-like values,
- normal UI font elsewhere.

Avoid:

- glassmorphism,
- large gradients,
- oversized rounded cards,
- excessive animation,
- consumer-dashboard appearance.

Animation only for:
- progress,
- panel transitions,
- selection feedback.

---

# 61. Main toolbar proposal

```text
[Open STEP] [Save]
|
[XY] [XZ] [YZ] [Pick Face] [Custom Plane]
|
[Compute Section]
[Validate]
|
[Export DXF]
[Send to Maxwell]  # disabled until future version
```

Use tooltips and shortcuts.

---

# 62. Status bar

Always display:

```text
Bodies: 34
Regions: 34
Errors: 0
Warnings: 2
Units: mm
Section: Custom @ 0.000 mm
```

When cursor is over 2D viewport:

```text
X: 23.425 mm
Y: -11.200 mm
```

---

# 63. Engineering tables

Where useful, use tables instead of cards.

Example component table:

| Export | Name | Source | Material | Regions | Status |
|---|---|---|---|---:|---|
| ✓ | Stator | Stator | Electrical Steel | 1 | PASS |
| ✓ | Cívka 01 | Cívka | Copper | 1 | PASS |

Support sorting/filtering.

---

# 64. Units

Internal geometry should use a clear unit convention.

Recommended:
- convert project coordinates to millimeters at application boundary,
- preserve source unit metadata,
- record conversion.

Never assume STEP is always mm.

UI may later display:
- mm,
- cm,
- m,
- inch.

DXF Maxwell preset defaults to mm.

---

# 65. Coordinate systems

Store:

- source CAD coordinate system,
- section plane coordinate system,
- exported 2D coordinate system.

Section plane basis:

```text
origin O
normal N
local X U
local Y V
```

Ensure right-handed basis.

Allow export origin mode:

- preserve projected CAD coordinates,
- section center to (0,0),
- machine detected center to (0,0),
- user-defined origin.

MVP mandatory:
- preserve,
- section bounding-box center.

---

# 66. Section plane auto-detection

Optional in early MVP, recommended after basic section works.

Candidate heuristics:

- principal bounding-box planes,
- large planar faces,
- repeated lamination mid-planes,
- planes producing high closed-region count,
- planes aligned with major geometry axes.

Never silently choose without showing the user.

Display suggestions:

```text
Suggested: Lamination mid-plane
Suggested: XY center
Suggested: Largest planar face
```

---

# 67. Electrical-machine specific extensions

Keep outside core geometry engine.

Future module:

```text
machine/
├── center_detection.py
├── airgap.py
├── slots.py
├── poles.py
├── symmetry.py
└── winding_groups.py
```

Possible features:

- rotor/stator separation,
- air-gap estimation,
- slot count,
- pole count,
- rotational symmetry,
- sector extraction.

These are domain inference features and should report confidence.

---

# 68. Symmetry workflow concept

Future UI:

```text
Machine symmetry detected:
12 repeating sectors
Sector angle: 30°

[Use full machine]
[Create 1/12 sector]
```

Never cut model automatically without explicit user action.

---

# 69. Material workflow concept

Future material mapping:

```text
Source group: Cívka
Suggested material: Copper
Confidence: user-defined rule

[Accept] [Change]
```

Avoid pretending color alone proves material.

Color can be used as a user-configured rule.

---

# 70. Rule-based mapping

Future:

```text
If source name contains "Cívka"
→ group = Windings
→ material tag = copper

If source name contains "Magnet"
→ group = Magnets
```

Rules stored per user/project.

MVP may implement simple name rules after core workflow works.

---

# 71. CLI future design

Future command:

```powershell
cad2maxwell convert motor.step `
  --plane XY `
  --offset 0 `
  --validate `
  --output motor.dxf
```

Useful for:
- batch conversion,
- regression testing,
- CI.

Core backend must therefore not depend on GUI.

---

# 72. Non-goals for MVP

Do NOT spend MVP time on:

- cloud accounts,
- collaborative editing,
- database server,
- mobile app,
- browser-only deployment,
- AI geometry generation,
- CFD,
- mechanical FEA,
- full CAD editing,
- mesh generation,
- full electromagnetic solver,
- direct DWG support if it complicates licensing,
- Linux packaging.

Focus:
Windows + STEP → validated Maxwell-ready 2D DXF.

---

# 73. Definition of done for each feature

Every feature is incomplete until it has:

1. typed implementation,
2. error handling,
3. tests,
4. documentation where appropriate,
5. no obvious UI dead-end,
6. no raw debug artifacts,
7. no hard-coded machine-specific assumptions.

---

# 74. Codex implementation protocol

Codex should work incrementally.

Before coding a milestone:

1. read this specification,
2. inspect repository,
3. write a short implementation plan,
4. identify impacted modules,
5. implement smallest coherent vertical slice,
6. run tests,
7. fix failures,
8. summarize changed files and remaining limitations.

Do not generate the entire application as one enormous commit.

---

# 75. Codex constraints

Codex must NOT:

- replace exact B-Rep sectioning with mesh slicing,
- approximate all arcs as polylines by default,
- place geometry logic in React,
- hard-code the user's example model dimensions,
- create a cloud architecture,
- introduce PostgreSQL,
- introduce Kubernetes,
- use Electron unless explicitly requested,
- rewrite geometry backend in Rust for MVP,
- silently drop metadata,
- silently repair significant geometry,
- use diacritics in Maxwell export names.

---

# 76. First implementation task for Codex

Start with Milestone 0 only.

Task:

> Create the monorepo foundation for CAD2Maxwell using Tauri + React + TypeScript for the desktop UI and Python 3.12 with FastAPI for the local geometry backend. Add strict linting/typechecking/testing, PowerShell dev/test scripts, a clean engineering workspace shell, and a backend health endpoint. Do not implement STEP geometry yet.

Expected result:

```text
pnpm install
.\scripts\dev.ps1
```

launches:

- desktop app,
- local backend,
- UI reports backend status.

UI initially contains:

- top application menu/toolbar,
- left model tree placeholder,
- center viewport placeholder,
- right properties placeholder,
- bottom status bar.

This establishes architecture before geometry complexity.

---

# 77. Second implementation task

After Milestone 0 is stable:

> Implement STEP import in the Python backend using OpenCascade/OCP. Enumerate solids, extract available names and colors, compute body bounding boxes, generate visualization meshes, and return a typed import summary. Display components in the React model tree and render the preview mesh in the 3D viewport. Do not implement sectioning yet.

Acceptance:

- test STEP loads,
- body count shown,
- source names shown when present,
- colors shown when present,
- 3D model selectable.

---

# 78. Third implementation task

> Implement exact section-plane geometry using OpenCascade. Support XY, XZ, YZ and custom plane definitions. Preserve component ownership, assemble section edges into closed wires, transform them into plane-local 2D coordinates and display them in the 2D viewport.

Acceptance:

- closed contours shown,
- holes preserved,
- per-component identity preserved,
- no mesh slicing.

---

# 79. Fourth implementation task

> Implement geometry validation and diagnostics, including open contours, duplicate edges, tiny edges, self-intersections where detectable and cross-component overlaps. Add clickable diagnostics in the UI.

---

# 80. Fifth implementation task

> Implement DXF export with one layer per component, deterministic ASCII-safe names, source colors where available, millimeter units, 1:1 scale, exact lines/arcs/circles when possible and a JSON sidecar manifest.

Test the exported DXF in ANSYS Maxwell 2D.

---

# 81. Final product principle

The application must optimize for:

```text
TRACEABILITY
+
GEOMETRIC CORRECTNESS
+
ENGINEERING UX
+
AUTOMATION
```

not merely "file conversion".

A successful CAD2Maxwell workflow should let an engineer answer:

- Which source CAD component created this 2D region?
- Is the contour geometrically valid?
- What name will Maxwell receive?
- What color/group/material mapping belongs to it?
- Can I reproduce the export deterministically?

If those answers are clear, the project is on the right path.
