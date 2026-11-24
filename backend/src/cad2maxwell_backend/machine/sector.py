import math
from dataclasses import dataclass

from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakePolygon
from OCP.BRepTools import BRepTools
from OCP.gp import gp_Dir, gp_Pln, gp_Pnt
from OCP.TopAbs import TopAbs_ShapeEnum
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS, TopoDS_Face, TopoDS_Wire

from cad2maxwell_backend.geometry.section import (
    ExactSectionWire,
    SectionError,
    _curve_2d,
    plane_frame,
)
from cad2maxwell_backend.models.sections import Point2, SectionPlane, SectionWire


@dataclass(frozen=True, slots=True)
class SectorComponent:
    wires: tuple[ExactSectionWire, ...]


def _wedge(
    plane: SectionPlane, center: Point2, order: int, start_deg: float, reach: float,
) -> TopoDS_Face:
    """A cutting tool only: a convex polygon covering the sector out to `reach`. Its radial
    edges are exact; the outer chord lies beyond the model and never reaches the result."""
    frame = plane_frame(plane)
    alpha = 2 * math.pi / order
    start = math.radians(start_deg)
    radius = reach / math.cos(alpha / 8)

    def point(x: float, y: float) -> gp_Pnt:
        return gp_Pnt(*(frame.origin[k] + x * frame.x_dir[k] + y * frame.y_dir[k]
                        for k in range(3)))

    polygon = BRepBuilderAPI_MakePolygon()
    polygon.Add(point(*center))
    for step in range(5):
        theta = start + step * alpha / 4
        polygon.Add(point(center[0] + radius * math.cos(theta),
                          center[1] + radius * math.sin(theta)))
    polygon.Close()
    surface = gp_Pln(gp_Pnt(*frame.origin), gp_Dir(*frame.normal))
    return BRepBuilderAPI_MakeFace(surface, polygon.Wire(), True).Face()


def _region_faces(plane: SectionPlane, wires: tuple[ExactSectionWire, ...]) -> list[TopoDS_Face]:
    """Outer wires become faces; each hole is subtracted from the smallest containing face."""
    frame = plane_frame(plane)
    surface = gp_Pln(gp_Pnt(*frame.origin), gp_Dir(*frame.normal))
    outers = [BRepBuilderAPI_MakeFace(surface, w.shape, True).Face()
              for w in wires if w.dto.role == "outer"]
    holes = [w for w in wires if w.dto.role == "hole"]
    if not holes:
        return outers
    from cad2maxwell_backend.geometry.region_metrics import region_metrics

    metrics = region_metrics("", wires, plane)
    result = []
    for metric in metrics:
        builder = BRepBuilderAPI_MakeFace(surface, wires[metric.outer_wire].shape, True)
        for hole in metric.hole_wires:
            builder.Add(TopoDS.Wire(wires[hole].shape.Reversed()))
        result.append(builder.Face())
    return result


def _wire_dto(wire: TopoDS_Wire, plane: SectionPlane, role: str) -> ExactSectionWire:
    from OCP.BRepTools import BRepTools_WireExplorer

    frame = plane_frame(plane)
    ordered = BRepTools_WireExplorer(wire)
    curves = []
    while ordered.More():
        curves.append(_curve_2d(ordered.Current(), frame))
        ordered.Next()
    return ExactSectionWire(
        shape=wire, dto=SectionWire(closed=wire.Closed(), role=role, curves=curves),  # type: ignore[arg-type]
    )


def cut_sector(
    components: dict[str, tuple[ExactSectionWire, ...]], plane: SectionPlane,
    center: Point2, order: int, start_deg: float, reach: float,
) -> dict[str, tuple[ExactSectionWire, ...]]:
    """Exact common of every section face with the wedge (BRepAlgoAPI); empty components drop."""
    wedge = _wedge(plane, center, order, start_deg, reach)
    result: dict[str, tuple[ExactSectionWire, ...]] = {}
    for component_id, wires in components.items():
        out: list[ExactSectionWire] = []
        for face in _region_faces(plane, wires):
            common = BRepAlgoAPI_Common(face, wedge)
            common.Build()
            if not common.IsDone():
                raise SectionError("OpenCascade could not cut the sector")
            faces = TopExp_Explorer(common.Shape(), TopAbs_ShapeEnum.TopAbs_FACE)
            while faces.More():
                piece = TopoDS.Face(faces.Current())
                outer = BRepTools.OuterWire_s(piece)
                out.append(_wire_dto(outer, plane, "outer"))
                wires_here = TopExp_Explorer(piece, TopAbs_ShapeEnum.TopAbs_WIRE)
                while wires_here.More():
                    inner = TopoDS.Wire(wires_here.Current())
                    if not inner.IsSame(outer):
                        out.append(_wire_dto(inner, plane, "hole"))
                    wires_here.Next()
                faces.Next()
        if out:
            result[component_id] = tuple(out)
    return result
