import math
from pathlib import Path
from typing import Any

import pytest
from fastapi.testclient import TestClient
from step_fixtures import named_colored_box, tilted_plate

from cad2maxwell_backend.api import create_app


def _import(client: TestClient, path: Path) -> dict[str, Any]:
    response = client.post("/api/v1/imports/step", json={"path": str(path)})
    assert response.status_code == 200
    return response.json()


def test_tilted_lamination_reports_its_sheet_plane(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = _import(client, tilted_plate(tmp_path / "plate.step"))
    component_id = imported["components"][0]["id"]

    response = client.get(
        f"/api/v1/imports/{imported['import_id']}/components/{component_id}/plane")

    assert response.status_code == 200
    payload = response.json()
    angle = math.radians(20)
    assert payload["component_id"] == component_id
    # Rotating +Z about +Y by 20 degrees gives (sin, 0, cos); the sign is canonical.
    assert payload["normal_xyz"] == pytest.approx((math.sin(angle), 0, math.cos(angle)), abs=1e-9)
    # The offset lies at mid-thickness: the plate spans 0..2 mm along its own normal.
    assert payload["offset_mm"] == pytest.approx(1.0, abs=1e-9)
    assert payload["planar_area_mm2"] == pytest.approx(2 * 40 * 30)


def test_custom_plane_from_component_cuts_the_full_lamination_profile(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = _import(client, tilted_plate(tmp_path / "plate.step"))
    component_id = imported["components"][0]["id"]
    plane = client.get(
        f"/api/v1/imports/{imported['import_id']}/components/{component_id}/plane").json()
    normal = plane["normal_xyz"]
    # Any unit vector perpendicular to the normal is a valid x direction.
    x_dir = (normal[2], 0.0, -normal[0])

    section = client.post("/api/v1/section", json={"import_id": imported["import_id"], "plane": {
        "kind": "custom", "origin_xyz": [value * plane["offset_mm"] for value in normal],
        "normal_xyz": normal, "x_dir_xyz": list(x_dir),
    }}).json()

    assert section["diagnostics"] == []
    [wire] = section["components"][0]["wires"]
    assert wire["closed"] is True and wire["role"] == "outer"
    points = [point for curve in wire["curves"] for point in (curve["start"], curve["end"])]
    width = max(p[0] for p in points) - min(p[0] for p in points)
    height = max(p[1] for p in points) - min(p[1] for p in points)
    assert sorted((round(abs(width), 6), round(abs(height), 6))) == [30.0, 40.0]


def test_unknown_component_returns_404(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = _import(client, named_colored_box(tmp_path / "coil.step"))

    response = client.get(f"/api/v1/imports/{imported['import_id']}/components/missing/plane")

    assert response.status_code == 404
    assert response.json()["error"]["code"] == "unknown_component"


def test_unknown_import_returns_404() -> None:
    response = TestClient(create_app()).get("/api/v1/imports/missing/components/any/plane")
    assert response.status_code == 404
    assert response.json()["error"]["code"] == "unknown_import"

