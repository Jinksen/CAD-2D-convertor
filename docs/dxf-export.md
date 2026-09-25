# DXF export

A draft DXF exporter is available through `python -m cad2maxwell_backend.cli`. It writes a 1:1 millimetre R2013 DXF and a versioned JSON manifest beside it. The exporter refuses empty sections, open or unclassified wires, unknown components, unsafe layer names, and existing output files. It uses one deterministic ASCII-safe layer per source component, preserving source color as DXF true color where available. Exact section lines become LINE, full circles become CIRCLE, circular arcs become ARC, and ellipses become ELLIPSE entities. No curve is flattened by default.

The manifest records source SHA-256, original names, hierarchy, color, section plane, wire roles, and import/section diagnostics. Its `draft_unvalidated` status is deliberate: cross-component overlap checks and an actual ANSYS Maxwell import are pending. Structural re-read and ezdxf audit are part of local verification, but they do not prove Maxwell region formation.

The final exporter will add validated Maxwell region formation, configurable origin modes, and material/group labels to this analytic draft path.

Export verification must include import into ANSYS Maxwell 2D, scale checks, closed-region checks, and separate object selectability.
