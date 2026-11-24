"""PyAEDT script writer. Emits Python text only; this package never imports pyaedt.

The generated script imports pyaedt when it runs. Material labels are user labels and are
written as TODO mappings, never as confirmed Maxwell library materials (spec section 16).
"""
from pprint import pformat

from cad2maxwell_backend.export.plan import ExportPlan
from cad2maxwell_backend.export.regions import ExportRegion

_TEMPLATE = '''"""CAD2Maxwell draft Maxwell 2D setup (generated; review before running).

Requires ANSYS Electronics Desktop and pyaedt. Place this file beside section.dxf.
"""
from pathlib import Path

import pyaedt  # imported only when this generated script runs

DXF = Path(__file__).with_name("section.dxf")
LAYER_MODE = __LAYER_MODE__
# One entry per region: name, DXF layer, groups, user material label.
REGIONS = __REGIONS__
# TODO: map each user label to a material that exists in YOUR Maxwell library, e.g.
# {"copper": "copper"}. Labels left as None are not assigned: nothing is guessed.
MATERIAL_MAP = __MATERIALS__


def main() -> None:
    maxwell = pyaedt.Maxwell2d(
        solution_type="MagnetostaticXY", version=None, non_graphical=False,
    )
    # TODO: adjust the call for your pyaedt version if import_dxf differs.
    maxwell.modeler.import_dxf(str(DXF), sorted({region["layer"] for region in REGIONS}))
    for region in REGIONS:
        label = region["material"]
        material = MATERIAL_MAP.get(label) if label else None
        if material is None:
            print(f"TODO material for {region['name']}: label={label!r}")
            continue
        # Objects take the DXF layer name; with layer_mode=group several regions share it.
        maxwell.assign_material(region["layer"], material)
    maxwell.release_desktop(close_projects=False, close_desktop=False)


if __name__ == "__main__":
    main()
'''


def write_pyaedt(regions: list[ExportRegion], plan: ExportPlan) -> str:
    rows = []
    for region in regions:
        group = region.body.primary_group
        rows.append({
            "name": region.name, "layer": region.body.layer,
            "groups": [plan.group_layers[name] for name in region.body.groups],
            "material": group.material if group else None,
        })
    materials = {label: None for label in sorted(
        {row["material"] for row in rows if row["material"]})}  # type: ignore[misc]
    return (_TEMPLATE
            .replace("__LAYER_MODE__", repr(plan.options.layer_mode))
            .replace("__REGIONS__", pformat(rows, sort_dicts=False, width=96))
            .replace("__MATERIALS__", pformat(materials, sort_dicts=False)))
