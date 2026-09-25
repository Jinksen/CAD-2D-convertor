from pathlib import Path

from fastapi.testclient import TestClient
from step_fixtures import named_colored_box, repeated_occurrences

from cad2maxwell_backend.api import create_app


def test_preview_mesh_matches_imported_box_and_component(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = client.post("/api/v1/imports/step", json={
        "path": str(named_colored_box(tmp_path / "box.step")),
    }).json()

    response = client.get(f"/api/v1/imports/{imported['import_id']}/preview")

    assert response.status_code == 200
    preview = response.json()
    assert preview["import_id"] == imported["import_id"]
    assert len(preview["components"]) == 1
    mesh = preview["components"][0]
    assert mesh["component_id"] == imported["components"][0]["id"]
    assert mesh["color"] == imported["components"][0]["source_color"]
    assert len(mesh["indices"]) == 36
    assert len(mesh["positions"]) == len(mesh["normals"]) == 36
    assert set(mesh["indices"]) == set(range(36))
    assert min(point[0] for point in mesh["positions"]) == 0
    assert max(point[0] for point in mesh["positions"]) == 10
    assert max(point[1] for point in mesh["positions"]) == 20
    assert max(point[2] for point in mesh["positions"]) == 30


def test_preview_preserves_repeated_component_identity(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = client.post("/api/v1/imports/step", json={
        "path": str(repeated_occurrences(tmp_path / "repeated.step")),
    }).json()

    response = client.get(f"/api/v1/imports/{imported['import_id']}/preview")

    assert response.status_code == 200
    meshes = response.json()["components"]
    assert [item["component_id"] for item in meshes] == [
        item["id"] for item in imported["components"]
    ]
    assert min(point[0] for point in meshes[0]["positions"]) == 0
    assert min(point[0] for point in meshes[1]["positions"]) == 10


def test_unknown_import_has_no_preview() -> None:
    response = TestClient(create_app()).get("/api/v1/imports/unknown/preview")

    assert response.status_code == 404
    assert response.json()["error"]["code"] == "unknown_import"
