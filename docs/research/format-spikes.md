# Format spikes (T53, #72)

Status: research only. No export code was added; each section ends with a recommendation. Probes were run against the repository's pinned ezdxf and OCP, on 2026-11-24.

## 1. Motor-CAD DXF import rules

Not verified. No Motor-CAD installation or vendor documentation was available to the spike, so nothing below is a claim about Motor-CAD. What can be said from the geometry side:

- Our DXF is 1:1 millimetres, closed analytic outlines (LINE, ARC, CIRCLE, ELLIPSE), one layer per component. Motor-CAD's geometry editor works from parametric templates; its DXF import is documented by Ansys as an import of curves that the user then assigns to regions, which suggests closed outlines are required and that region identity must come from layers or names.
- Open question to resolve with a licence: layer naming expected (by region type or free), unit handling (mm assumed or asked), whether ELLIPSE and ARC are accepted, and whether holes must be separate closed loops.

Recommendation: do not add a preset yet. Add one Motor-CAD item to the [Maxwell import checklist](../maxwell-import-checklist.md) style page, run our demo-motor export through a licensed Motor-CAD, record the accepted layer/closure rules, then decide whether a preset is just a `layer_mode` plus a name map.

## 2. 2D STEP export of section faces

Feasible, with one policy decision.

Probe: a planar face of a solid was added to an XCAF document with `ShapeTool.AddShape`, named through `TDataStd_Name`, and written by `STEPCAFControl_Writer`. The file contains the name (`Region_A`), `ADVANCED_FACE` and the exact surface (`CYLINDRICAL_SURFACE` for the cylinder side tested, so planar faces with exact circle/line edges are expected to round-trip analytically). Names and colours use the same path as the existing STEP fixtures.

What it needs:
- Build one planar face per region from the exact section wires (outer wire, holes added reversed). `machine/sector.py` already has this (`_region_faces`); it should move to a shared geometry helper when this is implemented.
- One XCAF product per component, one face per region, names from `export_name`, colours from the colour mode.
- Writer unit set to millimetres (`write.step.unit`), deterministic ordering from the existing export plan.

Policy decision: our own importer rejects face-only STEP (`STEP file contains no importable solids`). That is correct for sections, so a 2D STEP export is for COMSOL and other CAE only; it must not be advertised as re-importable into this application.

Cost: about one writer module (~100 lines) plus tests that read the file back with a raw `STEPCAFControl_Reader` and compare face areas and names. STEP byte order is not stable across OCCT runs (noted in `test_demo_motor.py`), so determinism tests must compare parsed content, not bytes.

Recommendation: worth doing after SVG and CSV (T48/T49). Open as its own ticket.

## 3. DXF R12 versus R2018 option

Current output is R2013 (`ezdxf.new(dxfversion="R2013")`). Probe with ezdxf:

| Version | LINE / ARC / CIRCLE | ELLIPSE | Notes |
|---------|--------------------|---------|-------|
| R12 (AC1009) | yes | no (`DXFVersionError`) | No `$INSUNITS` (ezdxf warns), no true colour (ACI 1-255 only), no LWPOLYLINE/SPLINE. |
| R2000 (AC1015) | yes | yes | Widest practical compatibility for older importers. |
| R2018 (AC1032) | yes | yes | Same entities as R2013; nothing our output needs. |

Consequences:
- R2018 gives no benefit; a "newer" option is not worth a switch.
- R12 cannot carry ellipses. Writing them would need polyline or spline approximation, which the project rules forbid unless an explicit export option asks for it. Circles and arcs are fine, so R12 is safe only for sections without ellipses; the exporter would have to refuse otherwise (not approximate silently).
- True colour (group 420) would be lost; ACI nearest-colour mapping is a lossy, explicit choice.
- Maxwell 2D reads current DXF; nothing in the Maxwell import checklist requires R12.

Recommendation: no R12 option now. If an old importer needs it, add `dxf_version: "R2013" | "R2000" | "R12"`, make R12 refuse ellipses with an error, map colours to ACI, and keep the manifest recording the version. R2000 is the cheap middle step and keeps ellipses and true colour.

## 4. PNG / PDF report snapshot

Not built. Findings:
- The Rune app renders the section view natively; a PNG snapshot is a frontend/GPU readback task, not a backend one, and must not approximate arcs differently from the display.
- A backend-only report is easy from the SVG writer (T48): SVG keeps arcs exact, and PNG/PDF can be produced by a rasteriser (cairosvg or resvg). Both add a dependency; neither belongs in the geometry core, so they would live behind an optional adapter.
- A PDF report would repeat the manifest (names, groups, areas from T49) beside the drawing; this is documentation, not an exchange format.

Recommendation: ship SVG first. Treat PNG/PDF as an optional report adapter after SVG and CSV, using an external rasteriser installed on demand, with golden-file tests on the SVG rather than on pixels.
