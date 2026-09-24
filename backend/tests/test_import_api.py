from pathlib import Path
from typing import Never

import pytest
from fastapi.testclient import TestClient
from step_fixtures import named_colored_box

from cad2maxwell_backend.api import create_app
from cad2maxwell_backend.services.import_service import ImportService
from cad2maxwell_backend.services.import_sessions import ImportSessions


def test_step_route_returns_typed_component_summary(tmp_path: Path) -> None:
    path = named_colored_box(tmp_path / "coil.step")

    response = TestClient(create_app()).post("/api/v1/imports/step", json={"path": str(path)})

    assert response.status_code == 200
    payload = response.json()
    assert payload["component_count"] == 1
    assert payload["source_unit"] == "millimetre"
    assert payload["components"][0]["source_name"] == "Cívka"
    assert payload["components"][0]["export_name"] == "Civka"
    assert payload["components"][0]["bounds_mm"]["max_xyz"] == [10.0, 20.0, 30.0]
    assert isinstance(payload["import_id"], str)


def test_invalid_paths_and_missing_files_map_to_expected_errors(tmp_path: Path) -> None:
    client = TestClient(create_app())
    directory = tmp_path / "folder.step"
    directory.mkdir()
    cases = [
        ("relative.step", 422, "invalid_import_path"),
        (str(tmp_path / "wrong.txt"), 422, "invalid_import_path"),
        (str(directory), 422, "invalid_import_path"),
        (str(tmp_path / "absent.step"), 404, "missing_import_file"),
    ]

    for path, status, code in cases:
        response = client.post("/api/v1/imports/step", json={"path": path})
        assert response.status_code == status
        assert response.json()["error"]["code"] == code


def test_unreadable_file_maps_to_403(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    path = tmp_path / "unreadable.step"
    path.write_text("STEP data", encoding="utf-8")
    original_open = Path.open

    def deny_target(self: Path, *args: object, **kwargs: object) -> object:
        if self == path:
            raise PermissionError("private path detail")
        return original_open(self, *args, **kwargs)

    monkeypatch.setattr(Path, "open", deny_target)
    response = TestClient(create_app()).post("/api/v1/imports/step", json={"path": str(path)})

    assert response.status_code == 403
    assert response.json()["error"]["code"] == "unreadable_import_file"
    assert "private path detail" not in response.text


def test_malformed_step_maps_to_400(tmp_path: Path) -> None:
    path = tmp_path / "bad.step"
    path.write_text("not a STEP file", encoding="utf-8")

    response = TestClient(create_app()).post("/api/v1/imports/step", json={"path": str(path)})

    assert response.status_code == 400
    assert response.json()["error"]["code"] == "rejected_step_file"


def test_unexpected_import_failure_is_generic(tmp_path: Path) -> None:
    path = tmp_path / "engine.step"
    path.write_text("contents hidden", encoding="utf-8")

    def explode(_path: Path) -> Never:
        raise RuntimeError("secret CAD text")

    app = create_app(ImportService(explode, ImportSessions()))
    response = TestClient(app, raise_server_exceptions=False).post(
        "/api/v1/imports/step", json={"path": str(path)}
    )

    assert response.status_code == 500
    assert response.json()["error"]["code"] == "internal_import_error"
    assert "secret CAD text" not in response.text
    assert "contents hidden" not in response.text


def test_tauri_origin_can_preflight_post() -> None:
    response = TestClient(create_app()).options(
        "/api/v1/imports/step",
        headers={"Origin": "http://tauri.localhost", "Access-Control-Request-Method": "POST"},
    )

    assert response.status_code == 200
    assert response.headers["access-control-allow-origin"] == "http://tauri.localhost"
    assert "POST" in response.headers["access-control-allow-methods"]
