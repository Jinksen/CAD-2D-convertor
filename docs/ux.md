# UX

The workspace follows a compact CAE/CAD panel layout: toolbar, model-tree area, central viewport, properties panel, and persistent status bar. The Milestone 0 shell provides these regions and clear empty/offline states.

The STEP import form accepts an absolute local path. On success, the model panel lists bodies in source order, selection shows source and export names, hierarchy, dimensions, and source units, and import diagnostics appear below the workspace. The status bar reports the imported body count. A failed import leaves the prior model visible and shows the backend error beside the form. Native file selection and 3D mesh preview are the next UI work.

After import, the 2D viewport offers an editable offset for the selected XY, XZ, or YZ plane. Compute Section requests exact curves from Python and draws component-colored outlines. Clicking a curve selects its source component. Errors remain visible without discarding the imported model. The viewport intentionally shows outlines without implying region validation or DXF readiness.

Selection will eventually synchronize across the model tree, 3D viewport, 2D section viewport, diagnostics, and properties. Controls must remain keyboard accessible with visible focus, status must never rely on color alone, and errors must be actionable. Expensive section operations will be debounced and provide progress rather than blocking the UI.

The visual direction is neutral graphite with restrained accent color, compact controls, and tabular engineering data instead of consumer-dashboard cards.
