from pathlib import Path

import pytest
from format_fixtures import brep_from_step, iges_from_step
from step_fixtures import inch_box, named_colored_box, repeated_occurrences

from cad2maxwell_backend.geometry.model_reader import read_model
from cad2maxwell_backend.geometry.xcaf_model import ModelImportError
from cad2maxwell_backend.services.import_service import (
    ImportService,
    InvalidImportPath,
)
from cad2maxwell_backend.services.import_sessions import ImportSessions


def test_iges_keeps_name_colour_and_exact_bounds(tmp_path: Path) -> None:
    model = read_model(iges_from_step(named_colored_box(tmp_path / "a.step"), tmp_path / "a.igs"))

    body = model.bodies[0]
    assert body.source_name is not None
    assert body.source_color == pytest.approx((204, 51, 26), abs=2)
    assert body.bounds_mm.max_xyz == pytest.approx((10, 20, 30))
    assert model.source_unit == "MM"
    assert model.to_mm_scale == 1.0


def test_iges_keeps_repeated_occurrences(tmp_path: Path) -> None:
    source = repeated_occurrences(tmp_path / "r.step")
    model = read_model(iges_from_step(source, tmp_path / "r.iges"))

    assert len(model.bodies) == 2
    assert model.bounds_mm.max_xyz[0] == pytest.approx(12)


def test_iges_is_deterministic(tmp_path: Path) -> None:
    path = iges_from_step(inch_box(tmp_path / "i.step"), tmp_path / "i.igs")
    first, second = read_model(path), read_model(path)

    assert [b.bounds_mm for b in first.bodies] == [b.bounds_mm for b in second.bodies]
    assert first.bodies[0].bounds_mm.max_xyz == pytest.approx((25.4, 50.8, 76.2))


def test_garbage_iges_is_rejected(tmp_path: Path) -> None:
    path = tmp_path / "bad.igs"
    path.write_text("not iges")
    with pytest.raises(ModelImportError):
        read_model(path)


def test_brep_bodies_get_deterministic_names_and_diagnostics(tmp_path: Path) -> None:
    path = brep_from_step(repeated_occurrences(tmp_path / "r.step"), tmp_path / "motor.brep")
    model = read_model(path)

    assert [b.source_name for b in model.bodies] == ["motor_1", "motor_2"]
    assert all(b.source_color is None for b in model.bodies)
    assert model.source_unit == "mm"
    assert {d.code for d in model.diagnostics} == {"brep_no_metadata", "assumed_source_unit"}
    assert model.bounds_mm.max_xyz[0] == pytest.approx(12)


def test_service_accepts_new_suffixes_and_rejects_others(tmp_path: Path) -> None:
    service = ImportService(read_model, ImportSessions())
    brep = brep_from_step(named_colored_box(tmp_path / "a.step"), tmp_path / "a.BREP")
    assert service.import_path(str(brep)).component_count == 1
    other = tmp_path / "a.obj"
    other.write_text("x")
    with pytest.raises(InvalidImportPath):
        service.import_path(str(other))


@pytest.mark.parametrize("suffix", [".igs", ".iges", ".brep"])
def test_upload_route_accepts_iges_and_brep(tmp_path: Path, suffix: str) -> None:
    from fastapi.testclient import TestClient

    from cad2maxwell_backend.api import create_app

    step = named_colored_box(tmp_path / "a.step")
    path = tmp_path / f"coil{suffix}"
    (brep_from_step if suffix == ".brep" else iges_from_step)(step, path)
    response = TestClient(create_app()).post(
        f"/api/v1/imports/step/file?filename=coil{suffix}", content=path.read_bytes(),
        headers={"Content-Type": "application/octet-stream"},
    )
    assert response.status_code == 200
    assert response.json()["component_count"] == 1
