import math
from dataclasses import dataclass

from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.BRepAlgoAPI import BRepAlgoAPI_Section
from OCP.BRepTools import BRepTools_WireExplorer
from OCP.GeomAbs import GeomAbs_CurveType
from OCP.gp import gp_Dir, gp_Pln, gp_Pnt
from OCP.ShapeAnalysis import ShapeAnalysis_FreeBounds
from OCP.TopAbs import TopAbs_Orientation, TopAbs_ShapeEnum
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS, TopoDS_Shape, TopoDS_Wire
from OCP.TopTools import TopTools_HSequenceOfShape

from cad2maxwell_backend.geometry.tolerance import DEFAULT_TOLERANCE
from cad2maxwell_backend.models.sections import (
    Circle2D,
    Curve2D,
    Ellipse2D,
    Line2D,
    Point2,
    Point3,
    SectionPlane,
    SectionWire,
)


class SectionError(ValueError):
    """Exact sectioning failed or produced a curve we cannot serialize."""


@dataclass(frozen=True, slots=True)
class PlaneFrame:
    origin: Point3
    normal: Point3
    x_dir: Point3
    y_dir: Point3

    def project(self, point: gp_Pnt) -> Point2:
        delta = (point.X() - self.origin[0], point.Y() - self.origin[1],
                 point.Z() - self.origin[2])
        return (sum(a * b for a, b in zip(delta, self.x_dir, strict=True)),
                sum(a * b for a, b in zip(delta, self.y_dir, strict=True)))

    def project_direction(self, direction: gp_Dir) -> Point2:
        vector = (direction.X(), direction.Y(), direction.Z())
        return (sum(a * b for a, b in zip(vector, self.x_dir, strict=True)),
                sum(a * b for a, b in zip(vector, self.y_dir, strict=True)))


def _normalize(vector: Point3) -> Point3:
    length = math.sqrt(sum(v * v for v in vector))
    return (vector[0] / length, vector[1] / length, vector[2] / length)


def plane_frame(plane: SectionPlane) -> PlaneFrame:
    if plane.kind == "XY":
        assert plane.offset_mm is not None
        return PlaneFrame((0, 0, plane.offset_mm), (0, 0, 1), (1, 0, 0), (0, 1, 0))
    if plane.kind == "XZ":
        assert plane.offset_mm is not None
        return PlaneFrame((0, plane.offset_mm, 0), (0, -1, 0), (1, 0, 0), (0, 0, 1))
    if plane.kind == "YZ":
        assert plane.offset_mm is not None
        return PlaneFrame((plane.offset_mm, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1))
    assert plane.origin_xyz and plane.normal_xyz and plane.x_dir_xyz
    normal = _normalize(plane.normal_xyz)
    x_dir = _normalize(plane.x_dir_xyz)
    y_dir = (normal[1] * x_dir[2] - normal[2] * x_dir[1],
             normal[2] * x_dir[0] - normal[0] * x_dir[2],
             normal[0] * x_dir[1] - normal[1] * x_dir[0])
    return PlaneFrame(plane.origin_xyz, normal, x_dir, y_dir)


def _curve_2d(edge: TopoDS_Shape, frame: PlaneFrame) -> Curve2D:
    adaptor = BRepAdaptor_Curve(TopoDS.Edge(edge))
    first = adaptor.FirstParameter()
    last = adaptor.LastParameter()
    if edge.Orientation() == TopAbs_Orientation.TopAbs_REVERSED:
        first, last = last, first
    kind = adaptor.GetType()
    if kind == GeomAbs_CurveType.GeomAbs_Line:
        return Line2D(start=frame.project(adaptor.Value(first)),
                      end=frame.project(adaptor.Value(last)))
    if kind == GeomAbs_CurveType.GeomAbs_Circle:
        circle = adaptor.Circle()
        return Circle2D(
            center=frame.project(circle.Location()), radius=circle.Radius(),
            x_axis=frame.project_direction(circle.XAxis().Direction()),
            y_axis=frame.project_direction(circle.YAxis().Direction()),
            start_parameter=first, end_parameter=last,
        )
    if kind == GeomAbs_CurveType.GeomAbs_Ellipse:
        ellipse = adaptor.Ellipse()
        return Ellipse2D(
            center=frame.project(ellipse.Location()),
            major_radius=ellipse.MajorRadius(), minor_radius=ellipse.MinorRadius(),
            x_axis=frame.project_direction(ellipse.XAxis().Direction()),
            y_axis=frame.project_direction(ellipse.YAxis().Direction()),
            start_parameter=first, end_parameter=last,
        )
    raise SectionError(f"Unsupported exact section curve: {kind.name}")


@dataclass(frozen=True, slots=True)
class ExactSectionWire:
    shape: TopoDS_Wire
    dto: SectionWire


def section_shape(shape: TopoDS_Shape, plane: SectionPlane) -> tuple[ExactSectionWire, ...]:
    frame = plane_frame(plane)
    section = BRepAlgoAPI_Section(
        shape, gp_Pln(gp_Pnt(*frame.origin), gp_Dir(*frame.normal)), False,
    )
    section.Build()
    if not section.IsDone():
        raise SectionError("OpenCascade could not compute the exact section")
    edges = TopTools_HSequenceOfShape()
    explorer = TopExp_Explorer(section.Shape(), TopAbs_ShapeEnum.TopAbs_EDGE)
    while explorer.More():
        edges.Append(explorer.Current())
        explorer.Next()
    if edges.Length() == 0:
        return ()
    wires = TopTools_HSequenceOfShape()
    # Shared vertices only: no implicit gap closing or geometry repair.
    ShapeAnalysis_FreeBounds.ConnectEdgesToWires_s(
        edges, DEFAULT_TOLERANCE.join_mm, True, wires,
    )
    result = []
    serialized_edges = 0
    for index in range(1, wires.Length() + 1):
        wire = TopoDS.Wire(wires.Value(index))
        count = 0
        wire_edges = TopExp_Explorer(wire, TopAbs_ShapeEnum.TopAbs_EDGE)
        while wire_edges.More():
            count += 1
            wire_edges.Next()
        ordered = BRepTools_WireExplorer(wire)
        curves = []
        while ordered.More():
            curves.append(_curve_2d(ordered.Current(), frame))
            ordered.Next()
        if len(curves) != count:
            raise SectionError("A section wire could not be traversed completely")
        serialized_edges += count
        result.append(ExactSectionWire(
            shape=wire, dto=SectionWire(closed=wire.Closed(), curves=curves),
        ))
    if serialized_edges != edges.Length():
        raise SectionError("Some section edges could not be assembled into wires")
    return tuple(result)
