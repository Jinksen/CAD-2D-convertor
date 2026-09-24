import pytest
from pydantic import ValidationError

from cad2maxwell_backend.models.imports import BoundingBox, ImportComponent, ImportResponse


def test_bounds_reject_nonfinite_and_reversed_coordinates() -> None:
    for minimum, maximum in [
        ((0, 0, float("nan")), (1, 1, 1)),
        ((2, 0, 0), (1, 1, 1)),
    ]:
        with pytest.raises(ValidationError):
            BoundingBox(min_xyz=minimum, max_xyz=maximum)


def test_component_rejects_invalid_rgb() -> None:
    with pytest.raises(ValidationError):
        ImportComponent(
            id="cmp_1",
            source_name=None,
            display_name="Body 1",
            export_name="Body_1",
            hierarchy_path=[],
            source_color=(256, 0, 0),
            body_ordinal=0,
            bounds_mm=BoundingBox(min_xyz=(0, 0, 0), max_xyz=(1, 1, 1)),
        )


def test_response_serializes_only_contract_fields() -> None:
    bounds = BoundingBox(min_xyz=(0, 0, 0), max_xyz=(1, 1, 1))
    response = ImportResponse(
        import_id="session-token",
        path="C:/part.step",
        sha256="a" * 64,
        source_unit="millimetre",
        to_mm_scale=1,
        bounds_mm=bounds,
        component_count=1,
        components=[
            ImportComponent(
                id="cmp_1",
                source_name=None,
                display_name="Body 1",
                export_name="Body_1",
                hierarchy_path=[],
                source_color=None,
                body_ordinal=0,
                bounds_mm=bounds,
            )
        ],
        diagnostics=[],
    )

    payload = response.model_dump(mode="json")
    assert payload["components"][0]["source_name"] is None
    assert payload["components"][0]["bounds_mm"]["max_xyz"] == [1.0, 1.0, 1.0]
    assert set(payload) == {
        "import_id", "path", "sha256", "source_unit", "to_mm_scale", "bounds_mm",
        "component_count", "components", "diagnostics",
    }
