"""FEMM Lua script writer (adapter output only; FEMM is never imported or run here).

Arcs use mi_addarc with the exact analytic angle (FEMM limits one arc to 180 degrees, so
larger arcs are split into equal exact parts). The `maxseg` argument only controls FEMM's
own arc discretisation for display and meshing; the stored geometry stays analytic.
"""
import math

from cad2maxwell_backend.export.plan import ExportPlan
from cad2maxwell_backend.export.primitives import Arc, Seg, number, split_arc, wire_primitives
from cad2maxwell_backend.export.regions import ExportRegion
from cad2maxwell_backend.models.sections import Point2
from cad2maxwell_backend.naming import assign_export_names

MAXSEG_DEGREES = 1


def _xy(point: Point2) -> str:
    return f"{number(point[0])},{number(point[1])}"


def write_femm(regions: list[ExportRegion], plan: ExportPlan) -> str:
    group_numbers = {group.name: index + 1 for index, group in enumerate(plan.options.groups)}
    lines = [
        "-- CAD2Maxwell draft section for FEMM (units: millimeters, planar problem).",
        "-- Geometry is exact: arcs keep their analytic angle. maxseg only affects FEMM display.",
        "-- Material names below are user labels, not FEMM library entries; define them in",
        "-- FEMM first and then uncomment the mi_setblockprop lines.",
        'newdocument(0)',
        'mi_probdef(0,"millimeters","planar",1e-8,1000,30)  -- depth 1000 mm is a placeholder',
    ]
    seen: set[str] = set()

    def node(point: Point2) -> None:
        text = _xy(point)
        if text not in seen:
            seen.add(text)
            lines.append(f"mi_addnode({text})")

    for region in regions:
        lines.append(f"-- region {region.name} (layer {region.body.layer})")
        for wire in [region.outer, *region.holes]:
            for item in wire_primitives(wire):
                if isinstance(item, Seg):
                    node(item.start)
                    node(item.end)
                    lines.append(f"mi_addsegment({_xy(item.start)},{_xy(item.end)})")
                    continue
                assert isinstance(item, Arc)
                pieces = max(1, math.ceil(item.sweep / math.pi - 1e-9))
                for piece in split_arc(item, pieces):
                    # mi_addarc runs counter-clockwise from the first to the second point.
                    first, second = (piece.start, piece.end) if piece.ccw \
                        else (piece.end, piece.start)
                    node(first)
                    node(second)
                    lines.append(f"mi_addarc({_xy(first)},{_xy(second)},"
                                 f"{number(math.degrees(piece.sweep))},{MAXSEG_DEGREES})")
        point = _xy(region.metric.interior_point)
        group = region.body.primary_group
        if group is None:
            name = region.body.source.export_name
        elif group.material == "custom" and group.material_note:
            name = assign_export_names([group.material_note])[0]
        else:
            name = group.material or plan.group_layers[group.name]
        number_ = group_numbers[group.name] if group else 0
        lines.append(f"mi_addblocklabel({point})")
        lines.append(f'-- mi_selectlabel({point}) mi_setblockprop("{name}",1,0,"<None>",0,'
                     f"{number_},1) mi_clearselected()")
        if group is not None:
            lines.append(f"mi_selectlabel({point})")
            lines.append(f"mi_setgroup({number_})")
            lines.append("mi_clearselected()")
    lines.append("mi_zoomnatural()")
    return "\n".join(lines) + "\n"
