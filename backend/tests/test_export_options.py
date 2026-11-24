import json
from pathlib import Path

import ezdxf
import pytest
from export_fixtures import demo_section, ids_named

from cad2maxwell_backend.export.dxf import DxfExportError, export_dxf
from cad2maxwell_backend.models.exports import ExportGroup, ExportOptions


def _groups(imported):  # type: ignore[no-untyped-def]
    return [
        ExportGroup(name="Magnets", members=ids_named(imported, "Magnet_"),
                    material="permanent_magnet"),
        ExportGroup(name="Cívky vinutí", members=ids_named(imported, "Coil_"),
                    material="copper"),
    ]


def _write(tmp_path: Path, options: ExportOptions, name: str = "s"):  # type: ignore[no-untyped-def]
    imported, section = demo_section()
    result = export_dxf(imported, section, tmp_path / f"{name}.dxf", options)
    return (ezdxf.readfile(result.dxf_path),
            json.loads(result.manifest_path.read_text(encoding="utf-8")))


def test_default_options_keep_component_layers_and_original_colours(tmp_path: Path) -> None:
    drawing, manifest = _write(tmp_path, ExportOptions())
    imported, _ = demo_section()
    assert {layer.dxf.name for layer in drawing.layers} >= {c.export_name for c in
                                                            imported.components}
    assert manifest["schema_version"] == 2
    assert manifest["layer_mode"] == "component" and manifest["color_mode"] == "original"
    assert manifest["groups"] == []
    assert all(c["layer"] == c["export_name"] for c in manifest["components"])


def test_group_mode_merges_members_on_ascii_group_layers(tmp_path: Path) -> None:
    imported, _ = demo_section()
    options = ExportOptions(layer_mode="group", groups=_groups(imported))
    drawing, manifest = _write(tmp_path, options)
    layers = {entity.dxf.layer for entity in drawing.modelspace()}
    assert {"Magnets", "Civky_vinuti", "Stator", "Rotor", "Shaft"} <= layers
    assert not any(name.startswith("Magnet_") for name in layers)
    assert [g["layer_name"] for g in manifest["groups"]] == ["Magnets", "Civky_vinuti"]
    assert manifest["groups"][1]["name"] == "Cívky vinutí"
    assert manifest["groups"][0]["material"] == "permanent_magnet"
    magnet = next(c for c in manifest["components"] if c["export_name"] == "Magnet_1")
    assert magnet["layer"] == "Magnets" and magnet["material_tag"] == "permanent_magnet"


def test_hybrid_mode_keeps_each_component_identifiable(tmp_path: Path) -> None:
    imported, _ = demo_section()
    drawing, _ = _write(tmp_path, ExportOptions(layer_mode="hybrid", groups=_groups(imported)))
    layers = {entity.dxf.layer for entity in drawing.modelspace()}
    assert "Magnets__Magnet_1" in layers and "Civky_vinuti__Coil_3" in layers
    assert "Stator" in layers


def test_body_in_two_groups_takes_the_first_group_and_lists_both(tmp_path: Path) -> None:
    imported, _ = demo_section()
    coil = ids_named(imported, "Coil_1")
    options = ExportOptions(layer_mode="group", groups=[
        ExportGroup(name="A", members=coil), ExportGroup(name="B", members=coil),
    ])
    drawing, manifest = _write(tmp_path, options)
    entry = next(c for c in manifest["components"] if c["component_id"] == coil[0])
    assert entry["layer"] == "A" and entry["groups"] == ["A", "B"]
    assert entry["primary_group"] == "A"
    assert "B" not in {e.dxf.layer for e in drawing.modelspace()}


def test_colour_modes(tmp_path: Path) -> None:
    imported, _ = demo_section()
    groups = _groups(imported)
    by_material, _ = _write(tmp_path, ExportOptions(color_mode="material", groups=groups), "m")
    copper = by_material.layers.get("Coil_1")
    assert copper.dxf.true_color == (184 << 16) + (115 << 8) + 51
    by_group, _ = _write(tmp_path, ExportOptions(color_mode="group", groups=groups), "g")
    assert by_group.layers.get("Magnet_1").dxf.true_color != \
        by_group.layers.get("Coil_1").dxf.true_color
    mono, _ = _write(tmp_path, ExportOptions(color_mode="monochrome", groups=groups), "o")
    assert not any(layer.dxf.hasattr("true_color") for layer in mono.layers)


def test_group_mode_original_colours_override_per_entity(tmp_path: Path) -> None:
    imported, _ = demo_section()
    drawing, _ = _write(tmp_path, ExportOptions(layer_mode="group", groups=_groups(imported)))
    assert all(layer.dxf.name != "Magnets" or layer.dxf.hasattr("true_color")
               for layer in drawing.layers)


def test_unknown_group_member_is_rejected(tmp_path: Path) -> None:
    imported, section = demo_section()
    options = ExportOptions(groups=[ExportGroup(name="X", members=["cmp_missing"])])
    with pytest.raises(DxfExportError, match="unknown component"):
        export_dxf(imported, section, tmp_path / "x.dxf", options)


def test_duplicate_group_names_and_labels_are_validated() -> None:
    with pytest.raises(ValueError):
        ExportOptions(groups=[ExportGroup(name="A"), ExportGroup(name="a")])
    with pytest.raises(ValueError):
        ExportGroup(name="A", material="unobtainium")  # type: ignore[arg-type]


def test_output_is_deterministic(tmp_path: Path) -> None:
    imported, _ = demo_section()
    options = ExportOptions(layer_mode="hybrid", color_mode="group", groups=_groups(imported))
    first = (tmp_path / "a")
    second = (tmp_path / "b")
    first.mkdir(), second.mkdir()
    a = export_dxf(*demo_section(), first / "s.dxf", options)
    b = export_dxf(*demo_section(), second / "s.dxf", options)
    assert a.manifest_path.read_bytes() == b.manifest_path.read_bytes()
    head = "$TDCREATE"
    assert a.dxf_path.read_text().split(head)[0] == b.dxf_path.read_text().split(head)[0]


def test_api_accepts_group_options_and_records_them(tmp_path: Path) -> None:
    import io
    from zipfile import ZipFile

    from fastapi.testclient import TestClient
    from step_fixtures import named_colored_box

    from cad2maxwell_backend.api import create_app

    client = TestClient(create_app())
    imported = client.post("/api/v1/imports/step", json={
        "path": str(named_colored_box(tmp_path / "coil.step"))}).json()
    section = client.post("/api/v1/section", json={
        "import_id": imported["import_id"], "plane": {"kind": "XY", "offset_mm": 15}}).json()
    body = {"import_id": imported["import_id"], "section_id": section["section_id"]}
    response = client.post("/api/v1/export/dxf", json={
        **body, "layer_mode": "group", "color_mode": "material",
        "groups": [{"name": "Winding", "members": [imported["components"][0]["id"]],
                    "material": "copper"}]})
    assert response.status_code == 200
    with ZipFile(io.BytesIO(response.content)) as archive:
        manifest = json.loads(archive.read("section.json"))
    assert manifest["groups"][0]["material"] == "copper"
    assert manifest["components"][0]["layer"] == "Winding"
    bad = client.post("/api/v1/export/dxf", json={**body, "layer_mode": "nope"})
    assert bad.status_code == 422
