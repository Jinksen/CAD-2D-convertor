# FreeCAD 3D and 2D workflow research

Researched 2026-09-25 from FreeCAD's official documentation and source. This note guides CAD2Maxwell's viewer work; it does not change the exact section or DXF contracts.

## What FreeCAD does

| Area | FreeCAD approach | Source |
| --- | --- | --- |
| CAD data | A Part shape remains a topological B-Rep; the view provider makes a separate display representation. | [ViewProviderPartExt source](https://github.com/FreeCAD/FreeCAD/blob/main/src/Mod/Part/Gui/ViewProviderExt.cpp) |
| 3D display | OpenCascade tessellates faces with linear and angular deflection. The view provider sends face triangles, edges, points, and normals to Coin3D. It retains per-face grouping for picking. | [ViewProviderPartExt source](https://github.com/FreeCAD/FreeCAD/blob/main/src/Mod/Part/Gui/ViewProviderExt.cpp) |
| 3D navigation | The viewer offers standard views, fit all/selection, orthographic or perspective camera, and a navigation cube. Objects can be selected from either the model tree or 3D view. | [View menu](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Std_View_Menu.md), [3D navigation](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Manual_Navigating_in_the_3D_view.md), [Selection view](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Selection_view.md) |
| True cross-section | Part Cross-sections takes a source shape, principal plane, and plane position to create actual section geometry. | [Part Cross-sections](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Part_CrossSections.md) |
| Drawing section | TechDraw SectionView cuts the shape, projects with hidden-line processing, and can show the cut surface with solid color or hatch. Its source separates the cut and projection stages. | [TechDraw SectionView](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/TechDraw_SectionView.md), [DrawViewSection source](https://github.com/FreeCAD/FreeCAD/blob/main/src/Mod/TechDraw/App/DrawViewSection.cpp), [DrawViewPart source](https://github.com/FreeCAD/FreeCAD/blob/main/src/Mod/TechDraw/App/DrawViewPart.cpp) |
| 2D display | Draft Shape2DView can present just cut lines or filled cut faces from a section plane. | [Draft Shape2DView](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Draft_Shape2DView.md) |

FreeCAD's temporary clipping plane changes the view; it does not by itself create section geometry. Likewise, a TechDraw hidden-line projection can include edges that lie behind the cutting plane. For Maxwell export, CAD2Maxwell should use the exact plane intersection and validated planar regions, not a screen clip or projected silhouette. See FreeCAD's [View menu](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Std_View_Menu.md), [Part Cross-sections](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Part_CrossSections.md), and [TechDraw SectionView](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/TechDraw_SectionView.md).

## CAD2Maxwell comparison

The current backend retains exact OCP bodies and generates separate display triangles. The Three.js tab supports orbit, zoom, and body selection; the 2D SVG tab draws exact analytic section curves and exports a draft DXF. The 3D DTO is JSON and capped at 50,000 triangles. The 2D display currently emphasizes outlines rather than filled material regions. The source of truth is already in the right place: Python/OpenCascade.

## Recommended next slices

1. **Link the section plane to both views.** Show a translucent XY/XZ/YZ plane at the exact requested offset in 3D. A drag or numeric edit should update one typed plane state; compute the exact 2D section after the interaction settles. Keep the graphical plane as a visual guide until the backend result arrives.
2. **Show validated regions in 2D.** Draw a fill for each component's outer loop with holes removed, while retaining analytic edge paths for selection and diagnostics. Fill should appear only when the backend has classified the loops. Use a visible style for open or invalid contours.
3. **Add engineering navigation.** Fit all, fit selection, orthographic XY/XZ/YZ, visibility/isolate controls, and a clear selected-body highlight are more useful here than full FreeCAD-style face editing. Keep tree, 3D, and 2D selection synchronized by component ID.
4. **Scale mesh delivery.** Replace large JSON arrays with a local binary mesh/GLB response or cache, with a component-ID mapping and coarse-to-fine preview. Keep mesh quality settings independent of section and DXF tolerances.
5. **Consider face/edge picking only when a task needs it.** FreeCAD maps render primitives back to B-Rep subelements; CAD2Maxwell currently needs reliable body selection first. Sub-element IDs would require a stable topology mapping contract.

For hardware, OpenCascade import, meshing, and exact sectioning use CPU. The WebView2/Three.js viewer uses the graphics stack for interactive display. No manual hardware profile is needed for the current workflow; the main scaling limit is the JSON mesh transport and rendering workload, not DXF calculation.
