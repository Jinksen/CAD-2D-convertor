"""Display-only triangulation of exact imported bodies."""

from math import sqrt

from OCP.BRep import BRep_Tool
from OCP.BRepMesh import BRepMesh_IncrementalMesh
from OCP.TopAbs import TopAbs_FACE, TopAbs_REVERSED
from OCP.TopExp import TopExp_Explorer
from OCP.TopLoc import TopLoc_Location
from OCP.TopoDS import TopoDS

from cad2maxwell_backend.domain.import_result import ImportedBody
from cad2maxwell_backend.models.imports import ImportResponse
from cad2maxwell_backend.models.preview import PreviewComponentMesh, PreviewMeshResponse

MAX_PREVIEW_TRIANGLES = 50_000


class PreviewMeshError(Exception):
    pass


def _triangle_normal(
    a: tuple[float, float, float],
    b: tuple[float, float, float],
    c: tuple[float, float, float],
) -> tuple[float, float, float] | None:
    ux, uy, uz = (b[axis] - a[axis] for axis in range(3))
    vx, vy, vz = (c[axis] - a[axis] for axis in range(3))
    nx, ny, nz = (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx)
    length = sqrt(nx * nx + ny * ny + nz * nz)
    if length == 0:
        return None
    return (nx / length, ny / length, nz / length)


def _mesh_body(body: ImportedBody, component_id: str, color: tuple[int, int, int] | None,
               deflection_mm: float, remaining_triangles: int) -> PreviewComponentMesh:
    mesher = BRepMesh_IncrementalMesh(body.shape, deflection_mm, False, 0.5, False)
    mesher.Perform()
    if not mesher.IsDone():
        raise PreviewMeshError("The 3D preview mesh could not be generated.")

    positions: list[tuple[float, float, float]] = []
    normals: list[tuple[float, float, float]] = []
    indices: list[int] = []
    faces = TopExp_Explorer(body.shape, TopAbs_FACE)
    while faces.More():
        face = TopoDS.Face_s(faces.Current())  # type: ignore[attr-defined]
        location = TopLoc_Location()
        triangulation = BRep_Tool.Triangulation_s(face, location)
        if triangulation is None or triangulation.NbTriangles() == 0:
            raise PreviewMeshError("A component face could not be rendered in 3D.")
        transform = location.Transformation()
        for triangle_index in range(1, triangulation.NbTriangles() + 1):
            node_ids = list(triangulation.Triangle(triangle_index).Get())
            if face.Orientation() == TopAbs_REVERSED:
                node_ids[1], node_ids[2] = node_ids[2], node_ids[1]
            points = [
                triangulation.Node(node_id).Transformed(transform) for node_id in node_ids
            ]
            triangle = [(point.X(), point.Y(), point.Z()) for point in points]
            normal = _triangle_normal(triangle[0], triangle[1], triangle[2])
            if normal is None:
                raise PreviewMeshError("A component contains a degenerate preview triangle.")
            if len(indices) // 3 >= remaining_triangles:
                raise PreviewMeshError(
                    "This model is too detailed for the current 3D preview. "
                    "Try a smaller STEP file."
                )
            start = len(positions)
            positions.extend(triangle)
            normals.extend((normal, normal, normal))
            indices.extend((start, start + 1, start + 2))
        faces.Next()
    if not indices:
        raise PreviewMeshError("A component has no renderable 3D faces.")
    return PreviewComponentMesh(component_id=component_id, color=color,
                                positions=positions, normals=normals, indices=indices)


def generate_preview(
    imported: ImportResponse, bodies: tuple[ImportedBody, ...],
) -> PreviewMeshResponse:
    if len(imported.components) != len(bodies):
        raise PreviewMeshError("The import session is inconsistent. Import the file again.")
    bounds = imported.bounds_mm
    diagonal = sqrt(sum((high - low) ** 2 for low, high in zip(
        bounds.min_xyz, bounds.max_xyz, strict=True,
    )))
    deflection_mm = max(diagonal * 0.002, 0.05)
    remaining = MAX_PREVIEW_TRIANGLES
    meshes: list[PreviewComponentMesh] = []
    for component, body in zip(imported.components, bodies, strict=True):
        mesh = _mesh_body(body, component.id, component.source_color, deflection_mm, remaining)
        meshes.append(mesh)
        remaining -= len(mesh.indices) // 3
    return PreviewMeshResponse(import_id=imported.import_id, components=meshes)
