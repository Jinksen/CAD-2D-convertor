# Usable DXF workflow

Goal: let an engineer choose a local STEP file in the desktop UI, compute an exact section, and download a traceable draft DXF for Maxwell.

1. Add a bounded, streaming STEP upload route for the system file chooser. Keep the existing absolute-path route and exact OpenCascade import path. Reject invalid filenames and oversized content. Test import and rejection behavior.
2. Retain typed import and section summaries alongside exact process-local shapes. Add a section export route that returns a ZIP containing analytic DXF and manifest without accepting an arbitrary output path. Test archive contents, session mismatches, and export failures.
3. Connect file choosing, section state, and export in React. Keep export disabled until a nonempty exportable section exists, show actionable errors, and label the download as a draft. Test the user flow.
4. Run focused tests, then backend and frontend quality gates. Update README with usage and limitations.

Follow-on: duplicate/tiny-edge checks, 3D preview, project save/load, and packaged-runtime verification. Positive-area cross-component overlap and planar face-validity checks are now part of section diagnostics. A successful rectangle import in Maxwell proves only the draft's basic scale and region formation.
