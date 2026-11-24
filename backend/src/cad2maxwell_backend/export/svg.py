"""SVG writer: 1 user unit = 1 mm, exact arcs, one path per region.

The section plane has y up, SVG has y down. Coordinates stay identical to the DXF; a
`scale(1 -1)` group flips the drawing and the viewBox is expressed in the flipped frame.
"""
import html
import math

from cad2maxwell_backend.export.plan import ExportPlan
from cad2maxwell_backend.export.primitives import Arc, Seg, number, split_arc, wire_primitives
from cad2maxwell_backend.export.regions import ExportRegion
from cad2maxwell_backend.models.sections import Ellipse2D, Point2, SectionWire


def _p(point: Point2) -> str:
    return f"{number(point[0])} {number(point[1])}"


def _ellipse_point(curve: Ellipse2D, t: float) -> Point2:
    a, b = curve.major_radius * math.cos(t), curve.minor_radius * math.sin(t)
    return (curve.center[0] + a * curve.x_axis[0] + b * curve.y_axis[0],
            curve.center[1] + a * curve.x_axis[1] + b * curve.y_axis[1])


def _subpath(wire: SectionWire) -> str:
    items = wire_primitives(wire, allow_ellipse=True)
    first = items[0]
    start = (first.start if isinstance(first, Seg | Arc)
             else _ellipse_point(first, first.start_parameter))
    parts = [f"M {_p(start)}"]
    for item in items:
        if isinstance(item, Seg):
            parts.append(f"L {_p(item.end)}")
        elif isinstance(item, Arc):
            # Two halves for a full circle (SVG cannot draw coincident endpoints).
            pieces = split_arc(item, 2) if item.sweep > math.pi * 1.999 else [item]
            for piece in pieces:
                large = int(piece.sweep > math.pi)
                r = number(piece.radius)
                parts.append(f"A {r} {r} 0 {large} {int(piece.ccw)} {_p(piece.end)}")
        else:
            assert isinstance(item, Ellipse2D)
            delta = item.end_parameter - item.start_parameter
            handed = item.x_axis[0] * item.y_axis[1] - item.x_axis[1] * item.y_axis[0]
            rotation = math.degrees(math.atan2(item.x_axis[1], item.x_axis[0]))
            ends = [item.start_parameter + delta * f for f in (0.5, 1.0)] \
                if abs(delta) > math.pi * 1.999 else [item.end_parameter]
            for end in ends:
                large = int(abs(delta) > math.pi and len(ends) == 1)
                parts.append(
                    f"A {number(item.major_radius)} {number(item.minor_radius)} "
                    f"{number(rotation)} {large} {int(delta * handed > 0)} "
                    f"{_p(_ellipse_point(item, end))}")
    parts.append("Z")
    return " ".join(parts)


def _hex(color: tuple[int, int, int] | None) -> str:
    return "#cccccc" if color is None else "#{:02x}{:02x}{:02x}".format(*color)


def write_svg(regions: list[ExportRegion], plan: ExportPlan) -> str:
    xs: list[float] = []
    ys: list[float] = []
    for region in regions:
        for wire in [region.outer, *region.holes]:
            for item in wire_primitives(wire, allow_ellipse=True):
                if isinstance(item, Arc):
                    xs += [item.center[0] - item.radius, item.center[0] + item.radius]
                    ys += [item.center[1] - item.radius, item.center[1] + item.radius]
                elif isinstance(item, Seg):
                    xs += [item.start[0], item.end[0]]
                    ys += [item.start[1], item.end[1]]
                else:
                    r = max(item.major_radius, item.minor_radius)
                    xs += [item.center[0] - r, item.center[0] + r]
                    ys += [item.center[1] - r, item.center[1] + r]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{number(x1 - x0)}mm" '
        f'height="{number(y1 - y0)}mm" viewBox="{number(x0)} {number(-y1)} '
        f'{number(x1 - x0)} {number(y1 - y0)}" data-units="mm" '
        f'data-layer-mode="{plan.options.layer_mode}">',
        '<g transform="scale(1 -1)" stroke="#000000" stroke-width="0.1" '
        'fill-rule="evenodd">',
    ]
    for region in regions:
        d = " ".join(_subpath(wire) for wire in [region.outer, *region.holes])
        groups = ",".join(region.body.groups)
        lines.append(
            f'<path id="{region.name}" data-layer="{region.body.layer}" '
            f'data-groups="{html.escape(groups)}" fill="{_hex(region.body.color)}" d="{d}"/>')
    lines += ["</g>", "</svg>", ""]
    return "\n".join(lines)
