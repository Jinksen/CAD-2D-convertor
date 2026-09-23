# Geometry

Geometry is intentionally absent from Milestone 0. Python with OpenCascade/OCP will be the sole geometric authority.

The future section pipeline will intersect source B-Rep bodies with a defined plane, assemble intersection edges into wires, transform those wires into a right-handed plane-local coordinate system, classify outer and inner loops, and retain source-component ownership. Analytic lines, arcs, circles, ellipses, and B-splines will be preserved where possible.

Preview meshes are visualization artifacts only. Mesh slicing is not an acceptable substitute for exact sectioning. Significant repairs must be reported to the user rather than applied silently, and tolerances will be centralized and unit-aware.
