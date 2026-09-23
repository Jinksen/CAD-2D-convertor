# DXF export

DXF export is planned for Milestone 4 and is not available in the foundation release.

The exporter will write 1:1 millimeter geometry with one deterministic, ASCII-safe layer or object name per component or logical group. Exact LINE, ARC, and CIRCLE entities will be used where the source section supports them. A versioned JSON sidecar manifest will map every exported object back to its source component, source name, color, hierarchy, and material/group labels.

Export verification must include import into ANSYS Maxwell 2D, scale checks, closed-region checks, and separate object selectability.
