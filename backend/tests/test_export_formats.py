import ast
import csv
import io
import math
import re
import xml.etree.ElementTree as ET
from pathlib import Path
from zipfile import ZipFile

import pytest
from export_fixtures import demo_exact, demo_section, ids_named

from cad2maxwell_backend.export.archive import EXTRA_FILES, export_archive
from cad2maxwell_backend.export.dxf import DxfExportError
from cad2maxwell_backend.models.exports import ExportGroup, ExportOptions

SRC = Path(__file__).resolve().parents[1] / "src"


def _options(**kw: object) -> ExportOptions:
    imported, _ = demo_section()
    groups = [
        ExportGroup(name="Magnets", members=ids_named(imported, "Magnet_"),
                    material="permanent_magnet"),
        ExportGroup(name="Cívky", members=ids_named(imported, "Coil_"), material="copper"),
    ]
    return ExportOptions(formats=list(EXTRA_FILES), groups=groups, **kw)  # type: ignore[arg-type]


def _archive(options: ExportOptions | None = None) -> dict[str, str]:
    imported, section = demo_section()
    data = export_archive(imported, section, options or _options(), demo_exact())
    with ZipFile(io.BytesIO(data)) as zf:
        return {n: zf.read(n).decode("utf-8") for n in zf.namelist()}


def test_default_archive_is_unchanged_and_extras_are_opt_in() -> None:
    imported, section = demo_section()
    with ZipFile(io.BytesIO(export_archive(imported, section))) as zf:
        assert sorted(zf.namelist()) == ["section.dxf", "section.json"]
    assert sorted(_archive()) == sorted(
        ["section.dxf", "section.json", *EXTRA_FILES.values()])


def test_region_formats_need_the_exact_section() -> None:
    imported, section = demo_section()
    with pytest.raises(DxfExportError, match="exact section"):
        export_archive(imported, section, _options(), None)


def test_every_extra_format_is_deterministic() -> None:
    first, second = _archive(), _archive()
    for name in EXTRA_FILES.values():
        assert first[name] == second[name]


def test_csv_table() -> None:
    rows = list(csv.DictReader(io.StringIO(_archive()["regions.csv"])))
    assert len(rows) == 15
    stator = next(r for r in rows if r["region"] == "Stator")
    assert stator["holes"] == "1" and float(stator["area_mm2"]) > 0
    magnet = next(r for r in rows if r["region"] == "Magnet_1")
    assert magnet["groups"] == "Magnets" and magnet["source_body"] == "Magnet_1"
    assert list(rows[0]) == ["region", "source_body", "groups", "area_mm2", "perimeter_mm",
                             "centroid_x_mm", "centroid_y_mm", "holes"]


def test_svg_keeps_exact_arcs_ids_and_holes() -> None:
    svg = _archive()["section.svg"]
    root = ET.fromstring(svg)
    paths = root.findall(".//{http://www.w3.org/2000/svg}path")
    ids = [p.get("id") for p in paths]
    assert len(ids) == 15 and len(set(ids)) == 15 and "Stator" in ids
    stator = next(p for p in paths if p.get("id") == "Stator")
    assert stator.get("d", "").count("M ") == 2  # outer plus one hole
    assert " A " in stator.get("d", "") and "scale(1 -1)" in svg
    assert next(p for p in paths if p.get("id") == "Coil_1").get("data-groups") == "Cívky"
    assert root.get("viewBox") is not None and root.get("data-units") == "mm"


def test_femm_script_uses_exact_arc_angles_and_block_labels() -> None:
    lua = _archive()["section.lua"]
    angles = [float(m) for m in re.findall(r"mi_addarc\([^)]*,([\d.e+-]+),1\)", lua)]
    assert angles and all(0 < a <= 180 + 1e-9 for a in angles)
    assert lua.count("mi_addblocklabel") == 15
    assert 'mi_setblockprop("copper"' in lua and "mi_setgroup(2)" in lua
    assert "maxseg" in lua and lua.startswith("-- CAD2Maxwell")


def test_gmsh_arcs_are_exact_short_circles_and_surfaces_have_holes() -> None:
    geo = _archive()["section.geo"]
    points = {int(i): (float(x), float(y)) for i, x, y in
              re.findall(r"Point\((\d+)\) = \{([^,]+), ([^,]+), 0, lc\}", geo)}
    arcs = re.findall(r"Circle\(\d+\) = \{(\d+), (\d+), (\d+)\}", geo)
    assert arcs
    for a, c, b in arcs:
        pa, pc, pb = points[int(a)], points[int(c)], points[int(b)]
        ra, rb = math.dist(pa, pc), math.dist(pb, pc)
        assert math.isclose(ra, rb, rel_tol=1e-7)
        cross = (pa[0] - pc[0]) * (pb[1] - pc[1]) - (pa[1] - pc[1]) * (pb[0] - pc[0])
        dot = (pa[0] - pc[0]) * (pb[0] - pc[0]) + (pa[1] - pc[1]) * (pb[1] - pc[1])
        assert 0 < math.atan2(cross, dot) % math.tau < math.pi  # ccw and under 180 degrees
    assert len(re.findall(r"Plane Surface", geo)) == 15
    assert re.search(r"Plane Surface\(\d+\) = \{\d+, \d+\};", geo)  # stator or rotor with a hole
    assert 'Physical Surface("Body_Stator")' in geo
    assert 'Physical Surface("Group_Civky")' in geo and 'Group_Magnets' in geo


def test_pyaedt_script_is_valid_python_and_core_never_imports_pyaedt() -> None:
    script = _archive()["section_pyaedt.py"]
    tree = ast.parse(script)
    assert any(isinstance(n, ast.Import) and n.names[0].name == "pyaedt" for n in ast.walk(tree))
    assert "MATERIAL_MAP" in script and "TODO" in script
    for path in SRC.rglob("*.py"):
        for node in ast.walk(ast.parse(path.read_text(encoding="utf-8"))):
            if isinstance(node, ast.Import | ast.ImportFrom):
                names = [a.name for a in node.names] + [getattr(node, "module", "") or ""]
                assert not any(n.split(".")[0] == "pyaedt" for n in names), path


def test_api_formats_selection(tmp_path: Path) -> None:
    from fastapi.testclient import TestClient
    from step_fixtures import named_colored_box

    from cad2maxwell_backend.api import create_app

    client = TestClient(create_app())
    imported = client.post("/api/v1/imports/step", json={
        "path": str(named_colored_box(tmp_path / "coil.step"))}).json()
    section = client.post("/api/v1/section", json={
        "import_id": imported["import_id"], "plane": {"kind": "XY", "offset_mm": 15}}).json()
    response = client.post("/api/v1/export/dxf", json={
        "import_id": imported["import_id"], "section_id": section["section_id"],
        "formats": ["dxf", "csv", "svg", "femm", "gmsh", "pyaedt"]})
    assert response.status_code == 200
    with ZipFile(io.BytesIO(response.content)) as zf:
        assert set(zf.namelist()) == {"section.dxf", "section.json", *EXTRA_FILES.values()}
        assert "<path" in zf.read("section.svg").decode()


def test_gmsh_full_circle_loops_close_on_shared_points() -> None:
    geo = _archive()["section.geo"]
    curves = {int(i): (int(a), int(b)) for i, a, b in
              re.findall(r"(?:Line|Circle)\((\d+)\) = \{(\d+), (?:\d+, )?(\d+)\}", geo)}
    for body in re.findall(r"Curve Loop\(\d+\) = \{([^}]*)\}", geo):
        ids = [int(i) for i in body.split(",")]
        ends = [curves[abs(i)] if i > 0 else curves[abs(i)][::-1] for i in ids]
        assert all(ends[k][1] == ends[(k + 1) % len(ends)][0] for k in range(len(ends)))
