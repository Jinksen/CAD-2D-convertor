import io
import json
from pathlib import Path
from zipfile import ZipFile

import ezdxf
import pytest
from fastapi.testclient import TestClient
from step_fixtures import named_colored_box, overlapping_occurrences

from cad2maxwell_backend.api import create_app


def test_file_chooser_upload_imports_exact_step(tmp_path: Path) -> None:
    source = named_colored_box(tmp_path / "coil.step")
    client = TestClient(create_app())

    response = client.post(
        "/api/v1/imports/step/file?filename=coil.step",
        content=source.read_bytes(),
        headers={"Content-Type": "application/octet-stream"},
    )

    assert response.status_code == 200
    assert response.json()["component_count"] == 1
    assert response.json()["path"] == "coil.step"
    section = client.post("/api/v1/section", json={
        "import_id": response.json()["import_id"], "plane": {"kind": "XY", "offset_mm": 15},
    })
    assert section.status_code == 200
    assert len(section.json()["components"][0]["wires"][0]["curves"]) == 4


def test_file_chooser_rejects_non_step_filename() -> None:
    response = TestClient(create_app()).post(
        "/api/v1/imports/step/file?filename=notes.txt", content=b"not step",
        headers={"Content-Type": "application/octet-stream"},
    )

    assert response.status_code == 422
    assert response.json()["error"]["code"] == "invalid_import_path"


def test_file_chooser_stops_oversized_upload(monkeypatch: pytest.MonkeyPatch) -> None:
    from cad2maxwell_backend import api

    monkeypatch.setattr(api, "MAX_UPLOAD_BYTES", 8)
    response = TestClient(create_app()).post(
        "/api/v1/imports/step/file?filename=large.step", content=b"123456789",
        headers={"Content-Type": "application/octet-stream"},
    )

    assert response.status_code == 413
    assert response.json()["error"]["code"] == "import_too_large"


def test_archive_contains_analytic_dxf_and_manifest(tmp_path: Path) -> None:
    client = TestClient(create_app())
    path = named_colored_box(tmp_path / "coil.step")
    imported = client.post("/api/v1/imports/step", json={"path": str(path)}).json()
    section = client.post("/api/v1/section", json={
        "import_id": imported["import_id"], "plane": {"kind": "XY", "offset_mm": 15},
    }).json()

    response = client.post("/api/v1/export/dxf", json={
        "import_id": imported["import_id"], "section_id": section["section_id"],
    })

    assert response.status_code == 200
    assert response.headers["content-type"] == "application/zip"
    with ZipFile(io.BytesIO(response.content)) as archive:
        assert set(archive.namelist()) == {"section.dxf", "section.json"}
        drawing = ezdxf.read(io.StringIO(archive.read("section.dxf").decode("utf-8")))
        manifest = json.loads(archive.read("section.json"))
    assert drawing.units == ezdxf.units.MM
    assert len(drawing.modelspace().query("LINE")) == 4
    assert manifest["validation_status"] == "draft_unvalidated"


def test_archive_rejects_section_from_another_import(tmp_path: Path) -> None:
    client = TestClient(create_app())
    path = named_colored_box(tmp_path / "coil.step")
    first = client.post("/api/v1/imports/step", json={"path": str(path)}).json()
    second = client.post("/api/v1/imports/step", json={"path": str(path)}).json()
    section = client.post("/api/v1/section", json={
        "import_id": first["import_id"], "plane": {"kind": "XY", "offset_mm": 15},
    }).json()

    response = client.post("/api/v1/export/dxf", json={
        "import_id": second["import_id"], "section_id": section["section_id"],
    })

    assert response.status_code == 409
    assert response.json()["error"]["code"] == "export_session_mismatch"


def test_overlap_diagnostic_blocks_draft_export(tmp_path: Path) -> None:
    client = TestClient(create_app())
    source = overlapping_occurrences(tmp_path / "overlap.step")
    imported = client.post("/api/v1/imports/step", json={"path": str(source)}).json()
    section = client.post("/api/v1/section", json={
        "import_id": imported["import_id"], "plane": {"kind": "XY", "offset_mm": 10},
    }).json()

    assert len(section["components"]) == 2
    assert section["diagnostics"][0]["code"] == "overlapping_regions"
    assert section["diagnostics"][0]["severity"] == "error"
    exported = client.post("/api/v1/export/dxf", json={
        "import_id": imported["import_id"], "section_id": section["section_id"],
    })
    assert exported.status_code == 422
    assert exported.json()["error"]["code"] == "export_not_ready"
