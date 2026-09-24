from pathlib import Path

import pytest
from OCP.TopoDS import TopoDS_Shape
from step_fixtures import inch_box, named_colored_box, repeated_occurrences, unnamed_box

from cad2maxwell_backend.geometry import step_importer
from cad2maxwell_backend.geometry.bounds import shape_bounds_mm
from cad2maxwell_backend.geometry.step_importer import StepImportError, read_step
from cad2maxwell_backend.models.imports import BoundingBox


def test_named_colored_box_preserves_exact_body_and_bounds(tmp_path: Path) -> None:
    model = read_step(named_colored_box(tmp_path / "coil.step"))

    assert len(model.bodies) == 1
    body = model.bodies[0]
    assert isinstance(body.shape, TopoDS_Shape)
    assert body.source_name == "Cívka"
    assert body.source_color == pytest.approx((204, 51, 26), abs=1)
    assert body.bounds_mm.min_xyz == pytest.approx((0, 0, 0))
    assert body.bounds_mm.max_xyz == pytest.approx((10, 20, 30))
    assert model.bounds_mm == body.bounds_mm


def test_repeated_occurrences_have_distinct_keys_and_locations(tmp_path: Path) -> None:
    model = read_step(repeated_occurrences(tmp_path / "repeated.step"))

    assert len(model.bodies) == 2
    first, second = model.bodies
    assert first.occurrence_key != second.occurrence_key
    assert first.hierarchy_path == ("Motor", "Magnet:1")
    assert second.hierarchy_path == ("Motor", "Magnet:2")
    assert first.bounds_mm.min_xyz[0] == pytest.approx(0)
    assert second.bounds_mm.min_xyz[0] == pytest.approx(10)
    assert second.bounds_mm.max_xyz[0] == pytest.approx(12)
    assert model.bounds_mm.max_xyz[0] == pytest.approx(12)


def test_inch_source_is_converted_to_millimetres(tmp_path: Path) -> None:
    model = read_step(inch_box(tmp_path / "inch.step"))

    assert model.source_unit is not None
    assert "inch" in model.source_unit.lower()
    assert model.to_mm_scale == pytest.approx(25.4)
    assert model.bodies[0].bounds_mm.max_xyz == pytest.approx((25.4, 50.8, 76.2))


def test_missing_optional_metadata_is_not_invented(tmp_path: Path) -> None:
    model = read_step(unnamed_box(tmp_path / "unnamed.step"))

    assert len(model.bodies) == 1
    assert model.bodies[0].source_color is None
    assert model.bodies[0].color_provenance is None


def test_malformed_step_is_rejected(tmp_path: Path) -> None:
    path = tmp_path / "bad.step"
    path.write_text("not a STEP file", encoding="utf-8")

    with pytest.raises(StepImportError):
        read_step(path)


def test_void_shape_has_no_serializable_bounds() -> None:
    assert shape_bounds_mm(TopoDS_Shape()) is None


def test_invalid_body_bounds_skip_only_that_body(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    path = repeated_occurrences(tmp_path / "one_invalid.step")
    original_bounds = step_importer.shape_bounds_mm
    calls = 0

    def first_body_invalid(shape: TopoDS_Shape) -> BoundingBox | None:
        nonlocal calls
        calls += 1
        return None if calls == 1 else original_bounds(shape)

    monkeypatch.setattr(step_importer, "shape_bounds_mm", first_body_invalid)
    model = read_step(path)

    assert len(model.bodies) == 1
    assert model.bodies[0].bounds_mm.min_xyz[0] == pytest.approx(10)
    assert [diagnostic.code for diagnostic in model.diagnostics] == ["invalid_body_bounds"]
