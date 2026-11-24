# DXF export

A draft DXF exporter is available through `python -m cad2maxwell_backend.cli` and the desktop section view. The desktop's **Export draft DXF** action downloads `section.zip` with `section.dxf` and `section.json`; the command line writes those files beside an explicit output path. The exporter writes a 1:1 millimetre R2013 DXF and a versioned JSON manifest. It refuses empty sections, open or unclassified wires, unknown components, unsafe layer names, and existing output files. It uses one deterministic ASCII-safe layer per source component, preserving source color as DXF true color where available. Exact section lines become LINE, full circles become CIRCLE, circular arcs become ARC, and ellipses become ELLIPSE entities. No curve is flattened by default.

The manifest records source SHA-256, original names, hierarchy, color, section plane, wire roles, and import/section diagnostics. Exact positive-area overlaps between components, invalid planar faces, and duplicate full edges within a component produce error diagnostics and block export. Duplicate checks compare exact coincident edge lengths, including curved edges; shared boundaries between separate components remain allowed. Partial coincident segments are not classified as full duplicates by this check. Edges shorter than the central `tiny_edge_mm` threshold (0.0001 mm by default) produce component-linked warnings and are preserved. No edge is repaired or deleted automatically. Its `draft_unvalidated` status remains deliberate: complex-model Maxwell import checks are pending. Structural re-read and ezdxf audit are part of local verification. A simple rectangular sample formed a filled region in Maxwell, but that does not prove complex assemblies will do so.

## Export options and manifest schema 2

`POST /api/v1/export/dxf` accepts optional `layer_mode`, `color_mode`, `groups` and `formats`.

- `layer_mode`: `component` (default, one layer per body), `group` (one ASCII-safe layer per group; bodies in no group keep their component layer) or `hybrid` (`<Group>__<Component>` for grouped bodies, so each body stays identifiable).
- `color_mode`: `original` (default, source colour), `group` (deterministic palette by group order), `material` (fixed colour per material label; groups without a label keep the source colour) or `monochrome` (no explicit colour). On a shared layer, bodies whose colour differs from the layer colour carry a per-entity true colour.
- `groups`: `{name, members: [component ids], material, material_note}`. Group names may contain any characters; layer names are derived by the same ASCII-safe, deterministic naming helper as components. Material labels are `air`, `copper`, `aluminum`, `electrical_steel`, `structural_steel`, `permanent_magnet`, `insulation`, `custom`. They are user-assigned, never inferred, and are not Maxwell library entries.
- A body that belongs to several groups takes its first group in request order for its layer and colour; every group is listed in the manifest. Unknown member ids are rejected with 422.
- `formats`: extra outputs added to the archive next to `section.dxf` and `section.json` (the DXF is always written; default `["dxf"]`).

Manifest `schema_version` is now 2. New keys: `layer_mode`, `color_mode`, `groups` (name, layer_name, material, material_note, members) and per component `layer`, `groups`, `primary_group`, `material_tag`. Default requests produce the same DXF as before.

## Additional formats

Set `formats` on the export request to add files to the archive (`section.dxf` and `section.json` are always included). Region-based formats read the exact section wires held by the service, so they work only for a section computed in the same session. A region is an outer wire minus the holes inside it; its name is the component export name, with `_01`, `_02` when a body has several regions. All writers are deterministic and use raw millimetres.

| `formats` | File | Notes |
|-----------|------|-------|
| `csv` | `regions.csv` | `region, source_body, groups, area_mm2, perimeter_mm, centroid_x_mm, centroid_y_mm, holes`; groups joined with `;`. |
| `svg` | `section.svg` | 1 user unit = 1 mm. Lines and exact `A` arc commands (a full circle is two arcs), ellipses as elliptical arcs, one `<path id=region>` per region with `fill-rule=evenodd` for holes. Fill follows `color_mode`. The section plane has y up and SVG y down, so the drawing sits in a `scale(1 -1)` group and the `viewBox` is given in that flipped frame; numbers equal the DXF coordinates. |
| `femm` | `section.lua` | `mi_addnode`, `mi_addsegment`, `mi_addarc` with the exact analytic angle (arcs over 180 degrees are split into equal exact parts, clockwise arcs have their end points swapped), one `mi_addblocklabel` at an interior point per region, `mi_setgroup` per export group. The `maxseg` argument (1 degree) only controls FEMM's display and meshing of arcs. `mi_setblockprop` lines are written commented out: block names are material labels or group names, not FEMM library materials. Ellipses are rejected. |
| `gmsh` | `section.geo` | `Point`, `Line`, `Circle(start, center, end)` (arcs split below 180 degrees, so a full circle is three arcs), `Curve Loop` and `Plane Surface` with hole loops, `Physical Surface("Body_<name>")` per body and `("Group_<name>")` per group. Coincident points and curves are shared, but bodies are not fused. `lc` is a placeholder mesh size. Ellipses are rejected. |
| `pyaedt` | `section_pyaedt.py` | Text only; the script imports `pyaedt` when run and the backend never does. It imports `section.dxf`, lists regions, layers and material labels, and leaves a `MATERIAL_MAP` of TODO entries so labels are not presented as Maxwell library materials. |

## Region metrics

`GET /api/v1/sections/{section_id}/regions` returns one entry per region (an outer wire minus the holes inside it): `area_mm2`, `perimeter_mm` (outer plus hole boundaries), `centroid`, `hole_count`, an `interior_point` guaranteed to lie in the material, and the wire indices in the section response. Values come from OpenCascade `BRepGProp` on the exact faces, not from polylines. They are display and table data only; geometry export never reads them.

## Validation tolerances

`POST /api/v1/section` accepts an optional `tolerance` object with `small_region_mm2` (1e-8 to 1), `sliver_ratio` (1e-6 to 0.05) and `join_mm` (1e-6 to 1e-3). Omitted fields keep the defaults; values outside these bounds are rejected with 422. The overrides take part in the section cache key and a non-default policy is returned in the section response and recorded in the manifest as `tolerance`. Default requests produce byte-identical output.

The final exporter will add validated Maxwell region formation, configurable origin modes, and material/group labels to this analytic draft path.

Export verification must include import into ANSYS Maxwell 2D, scale checks, closed-region checks, and separate object selectability.

The compiled desktop saves the archive through a Windows Save dialog and reports its output path. Cancelling is harmless; existing files are refused rather than overwritten. Native saves are capped at 64 MiB. The browser development harness still uses an anchor download. Extract the archive before opening the DXF in Maxwell.
