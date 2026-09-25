from pathlib import Path

from fastapi.testclient import TestClient
from step_fixtures import named_colored_box

from cad2maxwell_backend.api import create_app


def test_section_route_preserves_component_id_and_exact_wires(tmp_path: Path) -> None:
    app = create_app()
    client = TestClient(app)
    path = named_colored_box(tmp_path / "coil.step")
    imported = client.post("/api/v1/imports/step", json={"path": str(path)}).json()

    response = client.post("/api/v1/section", json={
        "import_id": imported["import_id"], "plane": {"kind": "XY", "offset_mm": 15},
    })

    assert response.status_code == 200
    payload = response.json()
    assert payload["components"][0]["component_id"] == imported["components"][0]["id"]
    assert payload["components"][0]["wires"][0]["closed"] is True
    assert payload["components"][0]["wires"][0]["role"] == "outer"
    assert len(payload["components"][0]["wires"][0]["curves"]) == 4
    assert {curve["type"] for curve in payload["components"][0]["wires"][0]["curves"]} == {"line"}
    assert payload["diagnostics"] == []
    exact = app.state.section_service.sections.get(payload["section_id"])
    assert exact is not None
    assert len(exact[imported["components"][0]["id"]]) == 1


def test_unknown_import_returns_404() -> None:
    response = TestClient(create_app()).post("/api/v1/section", json={
        "import_id": "missing", "plane": {"kind": "XY", "offset_mm": 0},
    })
    assert response.status_code == 404
    assert response.json()["error"]["code"] == "unknown_import"


def test_invalid_custom_plane_returns_422() -> None:
    response = TestClient(create_app()).post("/api/v1/section", json={
        "import_id": "missing", "plane": {
            "kind": "custom", "origin_xyz": [0, 0, 0],
            "normal_xyz": [0, 0, 1], "x_dir_xyz": [1, 0, 1],
        },
    })
    assert response.status_code == 422


def test_section_outside_model_reports_empty_result(tmp_path: Path) -> None:
    client = TestClient(create_app())
    path = named_colored_box(tmp_path / "coil.step")
    imported = client.post("/api/v1/imports/step", json={"path": str(path)}).json()

    response = client.post("/api/v1/section", json={
        "import_id": imported["import_id"], "plane": {"kind": "XY", "offset_mm": 100},
    })
    assert response.status_code == 200
    assert response.json()["components"] == []
    assert response.json()["diagnostics"][0]["code"] == "empty_section"
