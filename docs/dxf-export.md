# DXF export

A draft DXF exporter is available through `python -m cad2maxwell_backend.cli` and the desktop section view. The desktop's **Export draft DXF** action downloads `section.zip` with `section.dxf` and `section.json`; the command line writes those files beside an explicit output path. The exporter writes a 1:1 millimetre R2013 DXF and a versioned JSON manifest. It refuses empty sections, open or unclassified wires, unknown components, unsafe layer names, and existing output files. It uses one deterministic ASCII-safe layer per source component, preserving source color as DXF true color where available. Exact section lines become LINE, full circles become CIRCLE, circular arcs become ARC, and ellipses become ELLIPSE entities. No curve is flattened by default.

The manifest records source SHA-256, original names, hierarchy, color, section plane, wire roles, and import/section diagnostics. Exact positive-area overlaps between components and invalid planar faces produce error diagnostics and block export. Its `draft_unvalidated` status remains deliberate: duplicate and tiny-edge checks and complex-model Maxwell import checks are pending. Structural re-read and ezdxf audit are part of local verification. A simple rectangular sample formed a filled region in Maxwell, but that does not prove complex assemblies will do so.

The final exporter will add validated Maxwell region formation, configurable origin modes, and material/group labels to this analytic draft path.

Export verification must include import into ANSYS Maxwell 2D, scale checks, closed-region checks, and separate object selectability.
