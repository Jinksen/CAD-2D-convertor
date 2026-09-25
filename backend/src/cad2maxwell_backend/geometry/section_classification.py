from OCP.BRep import BRep_Tool
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepClass import BRepClass_FaceClassifier
from OCP.BRepGProp import BRepGProp
from OCP.gp import gp_Dir, gp_Pln, gp_Pnt
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_ShapeEnum, TopAbs_State
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS

from cad2maxwell_backend.geometry.section import ExactSectionWire, SectionError, plane_frame
from cad2maxwell_backend.geometry.tolerance import DEFAULT_TOLERANCE
from cad2maxwell_backend.models.sections import SectionPlane


def classify_wires(
    wires: tuple[ExactSectionWire, ...], plane: SectionPlane,
) -> tuple[str, ...]:
    """Classify closed section wires by exact planar containment parity."""
    frame = plane_frame(plane)
    surface = gp_Pln(gp_Pnt(*frame.origin), gp_Dir(*frame.normal))
    faces = []
    areas = []
    for wire in wires:
        if not wire.dto.closed:
            raise SectionError("An open section wire cannot bound a region")
        builder = BRepBuilderAPI_MakeFace(surface, wire.shape, True)
        if not builder.IsDone():
            raise SectionError("A section wire does not form a planar face")
        face = builder.Face()
        if not BRepCheck_Analyzer(face).IsValid():
            raise SectionError("A section wire self-intersects or forms an invalid face")
        properties = GProp_GProps()
        BRepGProp.SurfaceProperties_s(face, properties)
        area = abs(properties.Mass())
        if area <= DEFAULT_TOLERANCE.area_mm2:
            raise SectionError("A section wire has negligible area")
        faces.append(face)
        areas.append(area)

    roles = []
    for index, wire in enumerate(wires):
        vertices = TopExp_Explorer(wire.shape, TopAbs_ShapeEnum.TopAbs_VERTEX)
        if not vertices.More():
            raise SectionError("A section wire has no vertex")
        vertex = TopoDS.Vertex(vertices.Current())
        point = BRep_Tool.Pnt_s(vertex)
        containing = 0
        for candidate, face in enumerate(faces):
            if candidate == index or areas[candidate] <= areas[index]:
                continue
            state = BRepClass_FaceClassifier(face, point, DEFAULT_TOLERANCE.linear_mm).State()
            if state == TopAbs_State.TopAbs_ON:
                raise SectionError("Section contours touch ambiguously")
            if state == TopAbs_State.TopAbs_IN:
                containing += 1
        roles.append("hole" if containing % 2 else "outer")
    return tuple(roles)
