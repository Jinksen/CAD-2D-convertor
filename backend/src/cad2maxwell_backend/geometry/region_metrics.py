"""Exact region metrics from the section wires (display and export tables only)."""
import math

from OCP.Bnd import Bnd_Box
from OCP.BRep import BRep_Tool
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace
from OCP.BRepClass import BRepClass_FaceClassifier
from OCP.BRepGProp import BRepGProp
from OCP.gp import gp_Dir, gp_Pln, gp_Pnt
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_ShapeEnum, TopAbs_State
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS, TopoDS_Face

from cad2maxwell_backend.geometry.section import ExactSectionWire, SectionError, plane_frame
from cad2maxwell_backend.geometry.tolerance import DEFAULT_TOLERANCE
from cad2maxwell_backend.models.sections import Point2, RegionMetric, SectionPlane


def _props(face: TopoDS_Face) -> tuple[float, gp_Pnt]:
    properties = GProp_GProps()
    BRepGProp.SurfaceProperties_s(face, properties)
    return abs(properties.Mass()), properties.CentreOfMass()


def _perimeter(wire: ExactSectionWire) -> float:
    properties = GProp_GProps()
    BRepGProp.LinearProperties_s(wire.shape, properties)
    return abs(properties.Mass())


def _inside(face: TopoDS_Face, point: gp_Pnt) -> bool:
    state = BRepClass_FaceClassifier(face, point, DEFAULT_TOLERANCE.linear_mm).State()
    return state == TopAbs_State.TopAbs_IN


def region_metrics(
    component_id: str, wires: tuple[ExactSectionWire, ...], plane: SectionPlane,
) -> list[RegionMetric]:
    """One metric per outer wire; holes belong to the smallest outer wire containing them."""
    frame = plane_frame(plane)
    surface = gp_Pln(gp_Pnt(*frame.origin), gp_Dir(*frame.normal))
    faces: list[TopoDS_Face] = []
    areas: list[float] = []
    centres: list[gp_Pnt] = []
    for wire in wires:
        if not wire.dto.closed or wire.dto.role is None:
            raise SectionError("Region metrics need closed, classified wires")
        builder = BRepBuilderAPI_MakeFace(surface, wire.shape, True)
        if not builder.IsDone():
            raise SectionError("A section wire does not form a planar region")
        area, centre = _props(builder.Face())
        faces.append(builder.Face())
        areas.append(area)
        centres.append(centre)
    outers = [i for i, wire in enumerate(wires) if wire.dto.role == "outer"]
    owned: dict[int, list[int]] = {i: [] for i in outers}
    for index, wire in enumerate(wires):
        if wire.dto.role != "hole":
            continue
        vertices = TopExp_Explorer(wire.shape, TopAbs_ShapeEnum.TopAbs_VERTEX)
        vertex = TopoDS.Vertex(vertices.Current())
        point = BRep_Tool.Pnt_s(vertex)
        candidates = [i for i in outers if areas[i] > areas[index] and BRepClass_FaceClassifier(
            faces[i], point, DEFAULT_TOLERANCE.linear_mm).State() != TopAbs_State.TopAbs_OUT]
        if not candidates:
            raise SectionError("A section hole is not inside any outer wire")
        owned[min(candidates, key=lambda i: areas[i])].append(index)

    result = []
    for region_index, outer in enumerate(outers):
        holes = owned[outer]
        area = areas[outer] - sum(areas[h] for h in holes)
        moment = [areas[outer] * getattr(centres[outer], axis)() for axis in "XYZ"]
        for h in holes:
            for k, axis in enumerate("XYZ"):
                moment[k] -= areas[h] * getattr(centres[h], axis)()
        centre = gp_Pnt(*(m / area for m in moment))
        perimeter = _perimeter(wires[outer]) + sum(_perimeter(wires[h]) for h in holes)
        result.append(RegionMetric(
            component_id=component_id, region_index=region_index, outer_wire=outer,
            hole_wires=holes, area_mm2=area, perimeter_mm=perimeter,
            centroid=frame.project(centre), hole_count=len(holes),
            interior_point=_interior_point(
                faces[outer], [faces[h] for h in holes], centre, plane,
            ),
        ))
    return result


def _interior_point(
    outer: TopoDS_Face, holes: list[TopoDS_Face], centre: gp_Pnt, plane: SectionPlane,
) -> Point2:
    """Centroid when it lies in the material, else the first grid point that does."""
    frame = plane_frame(plane)

    def usable(point: gp_Pnt) -> bool:
        return _inside(outer, point) and not any(
            BRepClass_FaceClassifier(hole, point, DEFAULT_TOLERANCE.linear_mm).State()
            != TopAbs_State.TopAbs_OUT for hole in holes)

    if usable(centre):
        return frame.project(centre)
    box = Bnd_Box()
    BRepBndLib.Add_s(outer, box)
    low, high = box.CornerMin(), box.CornerMax()
    corners = [frame.project(gp_Pnt(x, y, z)) for x in (low.X(), high.X())
               for y in (low.Y(), high.Y()) for z in (low.Z(), high.Z())]
    x0, x1 = min(c[0] for c in corners), max(c[0] for c in corners)
    y0, y1 = min(c[1] for c in corners), max(c[1] for c in corners)
    c2 = frame.project(centre)
    steps = 40
    grid = [(x0 + (x1 - x0) * (i + 0.5) / steps, y0 + (y1 - y0) * (j + 0.5) / steps)
            for i in range(steps) for j in range(steps)]
    for x, y in sorted(grid, key=lambda p: (math.hypot(p[0] - c2[0], p[1] - c2[1]), p)):
        point = gp_Pnt(*(frame.origin[k] + x * frame.x_dir[k] + y * frame.y_dir[k]
                         for k in range(3)))
        if usable(point):
            return (x, y)
    raise SectionError("No interior point found for a region")
