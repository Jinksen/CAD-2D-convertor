# Exact section slice

Goal: turn session-owned STEP solids into traceable, exact OpenCascade section wires for XY, XZ, YZ, and custom planes. The local sample is a smoke test, never a source of hard-coded geometry behavior.

1. Add a typed plane request and a central millimetre tolerance policy. Reject non-finite, degenerate, or ambiguous custom bases. Test the coordinate transforms independently.
2. Intersect each retained B-Rep solid with the requested plane. Connect section edges into wires without closing significant gaps. Preserve component IDs; report open or unsupported topology. Test a box, a curved solid, multiple occurrences, and a miss.
3. Return analytic line, circle, and ellipse primitives in plane-local coordinates with stable wire order. Do not flatten them to polylines or pretend an unsupported curve is export-ready. Keep exact section shapes in a process-local session for later validation/export.
4. Expose `POST /api/v1/section` with typed errors for unknown imports and invalid planes. Test route behavior, run backend quality checks and the repository gate where available.
5. Smoke-test the local STEP sample at the user's chosen plane, compare extracted body and wire counts, and document diagnostics and remaining DXF/viewport work.

Follow-on authorized work: classify closed wires by exact containment and add a CLI-only analytic DXF draft with a sidecar manifest. Preserve import diagnostics and label the draft as unvalidated until Maxwell import and overlap checks are complete.
