"""Exact line and circular-arc primitives of a section wire, for the text writers."""
import math
from dataclasses import dataclass

from cad2maxwell_backend.export.plan import DxfExportError
from cad2maxwell_backend.models.sections import Circle2D, Ellipse2D, Line2D, Point2, SectionWire


@dataclass(frozen=True, slots=True)
class Seg:
    start: Point2
    end: Point2


@dataclass(frozen=True, slots=True)
class Arc:
    """Circular arc traversed from start to end; sweep is positive, ccw says the direction."""

    center: Point2
    radius: float
    start: Point2
    end: Point2
    start_angle: float  # radians, of the start point in the section plane
    sweep: float
    ccw: bool


def _point(curve: Circle2D, parameter: float) -> Point2:
    c, s = math.cos(parameter), math.sin(parameter)
    return (curve.center[0] + curve.radius * (c * curve.x_axis[0] + s * curve.y_axis[0]),
            curve.center[1] + curve.radius * (c * curve.x_axis[1] + s * curve.y_axis[1]))


def wire_primitives(
    wire: SectionWire, allow_ellipse: bool = False,
) -> list[Seg | Arc | Ellipse2D]:
    """Segments and arcs in traversal order; ellipses are returned as-is when allowed."""
    result: list[Seg | Arc | Ellipse2D] = []
    for curve in wire.curves:
        if isinstance(curve, Line2D):
            result.append(Seg(curve.start, curve.end))
        elif isinstance(curve, Circle2D):
            handed = curve.x_axis[0] * curve.y_axis[1] - curve.x_axis[1] * curve.y_axis[0]
            if abs(handed) < 1e-8:
                raise DxfExportError("A circle has a degenerate plane basis")
            delta = curve.end_parameter - curve.start_parameter
            if abs(delta) > math.tau + 1e-9:
                raise DxfExportError("A circular curve spans more than one turn")
            start = _point(curve, curve.start_parameter)
            result.append(Arc(
                center=curve.center, radius=curve.radius, start=start,
                end=_point(curve, curve.end_parameter),
                start_angle=math.atan2(start[1] - curve.center[1], start[0] - curve.center[0]),
                sweep=abs(delta), ccw=(delta * handed) > 0,
            ))
        elif allow_ellipse:
            result.append(curve)
        else:
            raise DxfExportError("This format supports lines and circular arcs only")
    return result


def split_arc(arc: Arc, pieces: int) -> list[Arc]:
    """Equal exact sub-arcs in traversal order."""
    step = arc.sweep / pieces * (1 if arc.ccw else -1)
    def at(index: int) -> Point2:
        angle = arc.start_angle + step * index
        return (arc.center[0] + arc.radius * math.cos(angle),
                arc.center[1] + arc.radius * math.sin(angle))
    points = [arc.start] + [at(i) for i in range(1, pieces)] + [arc.end]
    return [
        Arc(arc.center, arc.radius, points[i], points[i + 1],
            arc.start_angle + step * i, arc.sweep / pieces, arc.ccw)
        for i in range(pieces)
    ]


def number(value: float) -> str:
    """Deterministic compact decimal text without negative zero."""
    text = format(0.0 if abs(value) < 1e-9 else value, ".10g")  # below 1e-9 mm is noise
    return "0" if text == "-0" else text
