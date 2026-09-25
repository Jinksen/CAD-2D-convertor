# 3D preview slice

1. Generate per-component triangle meshes from retained exact OCP shapes in Python. Keep the mesh separate from sectioning and cap JSON transport size with an explicit error.
2. Expose a typed, process-local `GET /api/v1/imports/{import_id}/preview` contract with positions, normals, indices, component IDs, and source colors. Test a box, repeated components, and unknown import.
3. Render the preview with Three.js in the 3D tab. Fit the camera to the model, permit orbit/zoom, and select a component by clicking it. Keep the 2D section tab available.
4. Run focused and full checks, update README and architecture docs, and commit the vertical slice.

The preview triangles are display data only. Exact B-Rep shapes remain the authority for sections and DXF.
