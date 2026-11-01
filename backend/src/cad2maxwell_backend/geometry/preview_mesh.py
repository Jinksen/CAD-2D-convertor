"""Display-only triangulation of exact imported bodies.

The preview mesh is never used for sectioning or export; those work on the exact B-Rep.
"""

import sys
from array import array
from base64 import b64encode
from math import sqrt

from OCP.BRep import BRep_Tool
from OCP.BRepLib import BRepLib_ToolTriangulatedShape
from OCP.BRepMesh import BRepMesh_IncrementalMesh
from OCP.BRepTools import BRepTools
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_REVERSED
from OCP.TopExp import TopExp, TopExp_Explorer
from OCP.TopLoc import TopLoc_Location
from OCP.TopoDS import TopoDS
from OCP.TopTools import TopTools_IndexedMapOfShape

from cad2maxwell_backend.domain.import_result import ImportedBody
from cad2maxwell_backend.models.imports import ImportResponse
from cad2maxwell_backend.models.preview import PreviewComponentMesh, PreviewMeshResponse

MAX_PREVIEW_TRIANGLES = 50_000
COARSENING_FACTORS = (1.0, 2.0, 4.0, 8.0, 16.0)


class PreviewMeshError(Exception):
    pass


class _BodyMesh:
    def __init__(self) -> None:
        self.positions = array("f")
        self.normals = array("f")
        self.indices = array("I")
        self.edge_indices = array("I")

    @property
    def triangle_count(self) -> int:
        return len(self.indices) // 3


def _encode(values: array) -> str:  # type: ignore[type-arg]
    if sys.byteorder == "big":
        values = array(values.typecode, values)
        values.byteswap()
    return b64encode(values.tobytes()).decode("ascii")


def _mesh_body(body: ImportedBody, deflection_mm: float) -> _BodyMesh:
    BRepTools.Clean_s(body.shape)  # drop any finer cached triangulation so coarsening applies
    mesher = BRepMesh_IncrementalMesh(body.shape, deflection_mm, False, 0.5, False)
    mesher.Perform()
    if not mesher.IsDone():
        raise PreviewMeshError("The 3D preview mesh could not be generated.")

    edge_map = TopTools_IndexedMapOfShape()
    TopExp.MapShapes_s(body.shape, TopAbs_EDGE, edge_map)
    drawn_edges: set[int] = set()
    result = _BodyMesh()
    faces = TopExp_Explorer(body.shape, TopAbs_FACE)
    while faces.More():
        face = TopoDS.Face_s(faces.Current())  # type: ignore[attr-defined]
        location = TopLoc_Location()
        triangulation = BRep_Tool.Triangulation_s(face, location)
        if triangulation is None or triangulation.NbTriangles() == 0:
            raise PreviewMeshError("A component face could not be rendered in 3D.")
        BRepLib_ToolTriangulatedShape.ComputeNormals_s(face, triangulation)
        transform = location.Transformation()
        reversed_face = face.Orientation() == TopAbs_REVERSED
        offset = len(result.positions) // 3 - 1  # OCC node ids are 1-based
        for node_id in range(1, triangulation.NbNodes() + 1):
            point = triangulation.Node(node_id).Transformed(transform)
            normal = triangulation.Normal(node_id).Transformed(transform)
            sign = -1.0 if reversed_face else 1.0
            result.positions.extend((point.X(), point.Y(), point.Z()))
            result.normals.extend((sign * normal.X(), sign * normal.Y(), sign * normal.Z()))
        for triangle_index in range(1, triangulation.NbTriangles() + 1):
            a, b, c = triangulation.Triangle(triangle_index).Get()
            if reversed_face:
                b, c = c, b
            result.indices.extend((a + offset, b + offset, c + offset))

        edges = TopExp_Explorer(face, TopAbs_EDGE)
        while edges.More():
            edge = TopoDS.Edge_s(edges.Current())  # type: ignore[attr-defined]
            edge_id = edge_map.FindIndex(edge)
            polygon = BRep_Tool.PolygonOnTriangulation_s(edge, triangulation, location)
            if edge_id not in drawn_edges and polygon is not None:
                drawn_edges.add(edge_id)
                nodes = polygon.Nodes()
                for index in range(nodes.Lower(), nodes.Upper()):
                    result.edge_indices.extend(
                        (nodes.Value(index) + offset, nodes.Value(index + 1) + offset),
                    )
            edges.Next()
        faces.Next()
    if not result.indices:
        raise PreviewMeshError("A component has no renderable 3D faces.")
    return result


def generate_preview(
    imported: ImportResponse, bodies: tuple[ImportedBody, ...],
    max_triangles: int = MAX_PREVIEW_TRIANGLES,
) -> PreviewMeshResponse:
    if len(imported.components) != len(bodies):
        raise PreviewMeshError("The import session is inconsistent. Import the file again.")
    bounds = imported.bounds_mm
    diagonal = sqrt(sum((high - low) ** 2 for low, high in zip(
        bounds.min_xyz, bounds.max_xyz, strict=True,
    )))
    base_deflection_mm = max(diagonal * 0.002, 0.05)
    for factor in COARSENING_FACTORS:
        deflection_mm = base_deflection_mm * factor
        body_meshes: list[_BodyMesh] = []
        total = 0
        for body in bodies:
            body_meshes.append(_mesh_body(body, deflection_mm))
            total += body_meshes[-1].triangle_count
            if total > max_triangles:
                break
        if total <= max_triangles:
            break
    else:
        raise PreviewMeshError(
            "This model is too detailed for the 3D preview even at the coarsest "
            "display tolerance. Sectioning and export are not affected."
        )
    meshes = [
        PreviewComponentMesh(
            component_id=component.id, color=component.source_color,
            positions=_encode(mesh.positions), normals=_encode(mesh.normals),
            indices=_encode(mesh.indices), edge_indices=_encode(mesh.edge_indices),
        )
        for component, mesh in zip(imported.components, body_meshes, strict=True)
    ]
    return PreviewMeshResponse(
        import_id=imported.import_id, components=meshes,
        deflection_mm=deflection_mm, coarsened=factor > 1.0,
    )
