from array import array
from base64 import b64decode
from pathlib import Path
from typing import Any

from fastapi.testclient import TestClient
from OCP.BRepPrimAPI import BRepPrimAPI_MakeCylinder
from OCP.XCAFDoc import XCAFDoc_DocumentTool
from step_fixtures import _document, _write, named_colored_box, repeated_occurrences

from cad2maxwell_backend.api import create_app
from cad2maxwell_backend.geometry.preview_mesh import generate_preview
from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.services.import_service import ImportService
from cad2maxwell_backend.services.import_sessions import ImportSessions


def _decode(text: str, typecode: str) -> list[Any]:
    values = array(typecode)
    values.frombytes(b64decode(text))
    return values.tolist()


def _points(text: str) -> list[tuple[float, float, float]]:
    flat = _decode(text, "f")
    return [(flat[i], flat[i + 1], flat[i + 2]) for i in range(0, len(flat), 3)]


def _cylinder(path: Path) -> Path:
    document = _document()
    shapes = XCAFDoc_DocumentTool.ShapeTool_s(document.Main())
    shapes.AddShape(BRepPrimAPI_MakeCylinder(5, 10).Shape(), False)
    return _write(document, path)


def _import(client: TestClient, path: Path) -> dict[str, Any]:
    response = client.post("/api/v1/imports/step", json={"path": str(path)})
    assert response.status_code == 200
    return response.json()  # type: ignore[no-any-return]


def test_preview_mesh_matches_imported_box_and_component(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = _import(client, named_colored_box(tmp_path / "box.step"))

    response = client.get(f"/api/v1/imports/{imported['import_id']}/preview")

    assert response.status_code == 200
    preview = response.json()
    assert preview["import_id"] == imported["import_id"]
    assert preview["coarsened"] is False
    assert len(preview["components"]) == 1
    mesh = preview["components"][0]
    assert mesh["component_id"] == imported["components"][0]["id"]
    assert mesh["color"] == imported["components"][0]["source_color"]
    positions = _points(mesh["positions"])
    normals = _points(mesh["normals"])
    indices = _decode(mesh["indices"], "I")
    assert len(indices) == 36
    assert len(positions) == len(normals) == 24  # 4 shared nodes per face
    assert set(indices) == set(range(24))
    assert min(point[0] for point in positions) == 0
    assert max(point[0] for point in positions) == 10
    assert max(point[1] for point in positions) == 20
    assert max(point[2] for point in positions) == 30
    center = (5.0, 10.0, 15.0)
    for point, normal in zip(positions, normals, strict=True):
        outward = sum((point[axis] - center[axis]) * normal[axis] for axis in range(3))
        assert outward > 0


def test_box_preview_has_twelve_feature_edges(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = _import(client, named_colored_box(tmp_path / "box.step"))

    mesh = client.get(f"/api/v1/imports/{imported['import_id']}/preview").json()["components"][0]

    positions = _points(mesh["positions"])
    edge_indices = _decode(mesh["edge_indices"], "I")
    segments = {
        frozenset((positions[edge_indices[i]], positions[edge_indices[i + 1]]))
        for i in range(0, len(edge_indices), 2)
    }
    assert len(edge_indices) == 24
    assert len(segments) == 12
    assert all(len(segment) == 2 for segment in segments)


def test_cylinder_preview_has_smooth_normals(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = _import(client, _cylinder(tmp_path / "cylinder.step"))

    mesh = client.get(f"/api/v1/imports/{imported['import_id']}/preview").json()["components"][0]

    lateral = [
        (point, normal)
        for point, normal in zip(_points(mesh["positions"]), _points(mesh["normals"]), strict=True)
        if abs(normal[2]) < 1e-6
    ]
    # Smooth shading: each lateral node carries the exact surface normal (radial direction),
    # not a facet normal shared by a whole flat side.
    assert len({(round(n[0], 3), round(n[1], 3)) for _, n in lateral}) > 8
    for point, normal in lateral:
        assert abs(normal[0] - point[0] / 5) < 1e-4
        assert abs(normal[1] - point[1] / 5) < 1e-4


def test_small_triangle_budget_coarsens_instead_of_failing(tmp_path: Path) -> None:
    service = ImportService(read_step, ImportSessions())
    client = TestClient(create_app(service))
    imported = _import(client, _cylinder(tmp_path / "cylinder.step"))
    response = service.sessions.get_response(imported["import_id"])
    bodies = service.sessions.get(imported["import_id"])
    assert response is not None and bodies is not None
    fine = generate_preview(response, bodies)
    fine_triangles = len(b64decode(fine.components[0].indices)) // 12

    coarse = generate_preview(response, bodies, max_triangles=fine_triangles - 1)

    assert fine.coarsened is False
    assert coarse.coarsened is True
    assert coarse.deflection_mm > fine.deflection_mm
    assert len(b64decode(coarse.components[0].indices)) // 12 < fine_triangles


def test_preview_is_cached_per_import(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = _import(client, named_colored_box(tmp_path / "box.step"))
    url = f"/api/v1/imports/{imported['import_id']}/preview"

    first = client.get(url)
    second = client.get(url)

    assert first.status_code == second.status_code == 200
    assert first.content == second.content


def test_preview_preserves_repeated_component_identity(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = _import(client, repeated_occurrences(tmp_path / "repeated.step"))

    response = client.get(f"/api/v1/imports/{imported['import_id']}/preview")

    assert response.status_code == 200
    meshes = response.json()["components"]
    assert [item["component_id"] for item in meshes] == [
        item["id"] for item in imported["components"]
    ]
    assert min(point[0] for point in _points(meshes[0]["positions"])) == 0
    assert min(point[0] for point in _points(meshes[1]["positions"])) == 10


def test_unknown_import_has_no_preview() -> None:
    response = TestClient(create_app()).get("/api/v1/imports/unknown/preview")

    assert response.status_code == 404
    assert response.json()["error"]["code"] == "unknown_import"
