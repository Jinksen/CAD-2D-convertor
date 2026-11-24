"""Gmsh .geo writer: Point, Line, Circle(start, center, end), Curve Loop, Plane Surface.

Gmsh circle arcs must be smaller than 180 degrees, so larger arcs (and full circles) are
split into equal exact parts. Physical Surfaces exist per body (`Body_<name>`) and per
group (`Group_<name>`). Coincident points and curves are shared, but separate bodies are
not fused; the mesh is conformal only where boundaries are identical primitives.
"""
import math

from cad2maxwell_backend.export.plan import ExportPlan
from cad2maxwell_backend.export.primitives import Arc, Seg, number, split_arc, wire_primitives
from cad2maxwell_backend.export.regions import ExportRegion
from cad2maxwell_backend.models.sections import Point2, SectionWire


class _Builder:
    def __init__(self) -> None:
        self.lines: list[str] = []
        self.points: dict[tuple[str, str], int] = {}
        self.curves: dict[tuple[object, ...], int] = {}
        self.next_curve = 1

    def point(self, p: Point2) -> int:
        key = (number(round(p[0], 7)), number(round(p[1], 7)))  # snap near-equal ends
        if key not in self.points:
            self.points[key] = len(self.points) + 1
            self.lines.append(f"Point({self.points[key]}) = {{{key[0]}, {key[1]}, 0, lc}};")
        return self.points[key]

    def curve(self, key: tuple[object, ...], text: str) -> int:
        if key not in self.curves:
            self.curves[key] = self.next_curve
            self.lines.append(f"{text.replace('#', str(self.next_curve), 1)}")
            self.next_curve += 1
        return self.curves[key]

    def wire_ids(self, wire: SectionWire) -> list[int]:
        ids: list[int] = []
        for item in wire_primitives(wire):
            if isinstance(item, Seg):
                a, b = self.point(item.start), self.point(item.end)
                if ("L", b, a) in self.curves:
                    ids.append(-self.curves[("L", b, a)])
                else:
                    ids.append(self.curve(("L", a, b), f"Line(#) = {{{a}, {b}}};"))
                continue
            assert isinstance(item, Arc)
            for piece in split_arc(item, math.floor(item.sweep / math.pi) + 1):
                first, last = (piece.start, piece.end) if piece.ccw else (piece.end, piece.start)
                a, c, b = self.point(first), self.point(piece.center), self.point(last)
                cid = self.curve(("C", a, c, b), f"Circle(#) = {{{a}, {c}, {b}}};")
                ids.append(cid if piece.ccw else -cid)
        return ids


def write_gmsh(regions: list[ExportRegion], plan: ExportPlan) -> str:
    build = _Builder()
    surfaces: dict[str, int] = {}
    loop_count = 0
    body_surfaces: dict[str, list[int]] = {}
    group_surfaces: dict[str, list[int]] = {}
    surface_lines: list[str] = []
    for region in regions:
        loops = []
        for wire in [region.outer, *region.holes]:
            loop_count += 1
            ids = ", ".join(str(i) for i in build.wire_ids(wire))
            surface_lines.append(f"Curve Loop({loop_count}) = {{{ids}}};")
            loops.append(loop_count)
        surface = len(surfaces) + 1
        surfaces[region.name] = surface
        surface_lines.append(
            f"Plane Surface({surface}) = {{{', '.join(str(i) for i in loops)}}};")
        body_surfaces.setdefault(region.body.source.export_name, []).append(surface)
        for group in region.body.groups:
            group_surfaces.setdefault(plan.group_layers[group], []).append(surface)
    lines = [
        "// CAD2Maxwell draft section for Gmsh (units: millimeters).",
        "// Arcs are exact Circle entities (each < 180 degrees). lc is a mesh-size placeholder.",
        "lc = 1;",
        *build.lines, *surface_lines,
    ]
    for prefix, table in ("Body", body_surfaces), ("Group", group_surfaces):
        for name, members in table.items():
            lines.append(
                f'Physical Surface("{prefix}_{name}") = {{{", ".join(map(str, members))}}};')
    return "\n".join(lines) + "\n"
