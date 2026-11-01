# Maxwell 2D import verification checklist

Manual check of v0.1 acceptance criteria 13-15 (spec section 47). Until it has been run on a real
model, exports keep the manifest status `draft_unvalidated`.

Needs ANSYS Maxwell 2D (GUI). CAD2Maxwell itself does not need ANSYS.

## Prepare

1. Start CAD2Maxwell and import `docs/demo/demo-motor.step` (15 bodies), or your own STEP file.
2. Select the XY plane, offset 25 mm (demo mid-height) and press **Compute section**.
3. Confirm the Messages pane shows no errors, then **Export draft DXF**. Unzip `section.zip` into a
   new folder: `section.dxf` and `section.json`.
4. Note the expected values from `section.json`: `components` (one entry per body), each with
   `export_name` (= DXF layer name) and `wires` (roles `outer` / `hole`).

## Steps

| # | Step in Maxwell 2D | Expected result |
|---|--------------------|-----------------|
| 1 | Create a new project and a Maxwell 2D design (Magnetostatic or Transient), geometry mode XY. Set the model units to mm. | Design opens; units are mm. |
| 2 | Modeler > Import... choose `section.dxf`. In the DXF import dialog select all layers, keep "Heal" off for the first try, and use mm. | Import finishes without an error. |
| 3 | Compare the imported size with the section's bounding box (demo: stator outer diameter 120 mm). | Size and position match the CAD2Maxwell section; no scaling. |
| 4 | Look at the Modeler history tree. | Closed contours are present as sheets/objects, not only as loose lines. |
| 5 | Click a few objects (stator, a magnet, a coil side) in the graphics window. | Each body is selectable on its own. |
| 6 | Count the objects whose name or layer matches a manifest `export_name`. | One object per manifest component; counts agree (demo: 15). |
| 7 | Check the holes (demo: stator bore with slots, rotor bore). | Hole wires are voids, not filled; the shaft is a separate object. |
| 8 | Select two touching bodies (rotor and shaft). Do not unite them. | They stay separate objects. |
| 9 | Assign a material to one object and run **Validate**. | Validation shows no geometry errors. |
| 10 | Compare object names with `section.json` `export_name` values. | Names match the manifest mapping (ASCII, no diacritics). |

If step 4 gives only lines, try the import option that converts closed polylines to sheets and
record it in the table below; do not edit the DXF by hand.

## Results

Fill in one copy of this table per verified model.

| Field | Value |
|-------|-------|
| Date | |
| Tester | |
| CAD2Maxwell version / commit | |
| Maxwell version | |
| STEP file | |
| Plane and offset | |

| Step | Pass / Fail | Notes (object counts, messages, screenshot file) |
|------|-------------|--------------------------------------------------|
| 1 | | |
| 2 | | |
| 3 | | |
| 4 | | |
| 5 | | |
| 6 | | |
| 7 | | |
| 8 | | |
| 9 | | |
| 10 | | |

## Acceptance mapping

| Criterion | Steps | Verdict |
|-----------|-------|---------|
| 13. DXF opens in ANSYS Maxwell 2D | 1-3 | |
| 14. Regions usable as separate selectable Maxwell objects | 4-9 | |
| 15. Sidecar manifest maps source component to exported object | 6, 10 | |

When all steps pass, record the Maxwell version and model in `docs/dxf-export.md` and only then
consider changing the manifest `validation_status`.
