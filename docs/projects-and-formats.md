# Projects, model organization and file formats

Status: proposal plus the first slice, implemented in the same PR (0.3).

## Implemented in 0.3

| Area | Change |
|------|--------|
| Model tree | Bodies nest in collapsible folders that follow the STEP assembly structure. Levels shared by every body (usually the root assembly) are skipped. While the filter is active, every folder is expanded so no match hides. |
| Groups | User groups such as "Magnets" or "Phase A coils". A body can be in several groups. Create a group from the selected body, add or remove bodies in the body pane, and rename, hide, isolate or delete it in the group pane. V and I work on the selected group. |
| Project file | Groups and collapsed folders are saved as optional fields, so 0.2 can still open the file; it ignores those fields. The STEP path is also stored relative to the project folder when the STEP lies inside it, so a moved or shared project folder still finds its model. Saving writes a temporary file and renames it over the target, so a crash cannot leave a half-written project. |
| Desktop | Installed builds start the geometry service on a free loopback port, so several windows can run at once. |

Groups are display organization only. They do not change sectioning, validation, export names or the DXF.

## Next: project file

In order of value:

1. **Unsaved-changes marker and prompt.** Show `*` in the title after a change, and ask before closing or opening another model. The project already serializes the full session, so comparing the last-saved text with `project_file()` is enough.
2. **Several named sections per project.** For example "Mid stack z = 25 mm" and "End winding z = 48 mm". Each one stores plane, offset and the export path it was last written to. This is the most common reason to reopen a project.
3. **Groups in the export.** Write each body's groups to the JSON manifest and offer DXF layers per group (ASCII-safe names, as for export names). In Maxwell, a layer per group is one material or excitation assignment instead of one per body. This needs a backend change and a manifest schema version bump.
4. **Recent projects.** A short list in the ribbon, stored in preferences, not in the project.
5. **Recovery file.** Write `<name>.c2mproj.autosave` every few minutes and offer it on the next open if it is newer than the project.
6. **Published schema.** A JSON Schema for `.c2mproj` in `docs/`, with the rule already in use: new fields are optional, and the version only increases when an older app would misread the file.

Not planned: storing geometry or section results in the project. The project stays a small, readable reference to the STEP file plus settings, and the STEP hash detects changes.

## Next: file formats

The core constraint applies to every format: exact B-Rep in, analytic curves out. Mesh formats cannot carry exact geometry, so they are not section sources.

### Import

| Format | Verdict | Notes |
|--------|---------|-------|
| STEP AP203/214/242 | Supported | Names, colours and assembly structure through XCAF. |
| IGES (`.igs`, `.iges`) | Recommended next | Open CASCADE reads it with `IGESCAFControl_Reader`, including names, colours and levels. It is the second most common neutral CAD format for older motor models. About one reader function plus tests. |
| OCCT BREP (`.brep`) | Cheap, low value | Exact geometry but no names or colours. Useful for test fixtures and FreeCAD exports. |
| Inventor `.ipt` / `.iam` | Adapter only | Through the planned Inventor COM add-in that exports STEP. It is never a dependency of the core. |
| Parasolid, ACIS `.sat`, JT | Not possible | Open CASCADE has no free readers for them. Ask users for STEP. |
| STL, OBJ, 3MF | Rejected | Triangle meshes. Sectioning them would be mesh slicing. |

### Export

| Format | Verdict | Notes |
|--------|---------|-------|
| DXF + JSON manifest | Supported | 1:1 in millimetres, analytic arcs, deterministic names. |
| DXF with layers per group | Recommended next | See "Groups in the export" above. |
| Region table (CSV) | Recommended | Region name, source body, groups, area, perimeter and centroid. Useful for checking and reports, and computed from exact geometry in the backend. |
| SVG | Useful | Documentation and reports. Arcs map to SVG arcs exactly. Same geometry as the DXF, with a different writer. |
| PyAEDT script (`.py`) | Adapter | Builds the regions in Maxwell 2D, names them and assigns materials per group. It belongs behind an adapter. PyAEDT stays optional and is never required to run the core. |
| Motor-CAD DXF | Check first | Motor-CAD imports DXF. Verify which layer and closure rules it needs before adding a preset. |
| ACIS `.sat` | Not possible | No free writer in Open CASCADE. DXF covers Maxwell 2D import. |

### Suggested order

1. Groups in the manifest and as DXF layers, since they make the groups added in 0.3 useful in Maxwell.
2. IGES import.
3. Unsaved-changes prompt and named sections.
4. Region table CSV and SVG.
5. The PyAEDT adapter, after DXF import has been verified with the [Maxwell import checklist](maxwell-import-checklist.md).
