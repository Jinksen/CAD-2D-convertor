from typing import Any

import pytest
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox

from cad2maxwell_backend.domain.import_result import ImportedBody
from cad2maxwell_backend.geometry.bounds import misses_plane
from cad2maxwell_backend.geometry.section import section_shape
from cad2maxwell_backend.models.imports import BoundingBox
from cad2maxwell_backend.models.sections import SectionPlane, SectionRequest
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.limits import MAX_IMPORTS, MAX_SECTIONS
from cad2maxwell_backend.services.section_service import SectionService, UnknownImport
from cad2maxwell_backend.services.section_sessions import SectionSessions


def _body(name: str, size: tuple[float, float, float], z: float = 0) -> ImportedBody:
    shape = BRepPrimAPI_MakeBox(*size).Shape()
    if z:
        from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform
        from OCP.gp import gp_Trsf, gp_Vec

        move = gp_Trsf()
        move.SetTranslation(gp_Vec(0, 0, z))
        shape = BRepBuilderAPI_Transform(shape, move, True).Shape()
    return ImportedBody(
        shape=shape, occurrence_key=name, solid_ordinal=0,
        source_id=None, source_name=name, name_provenance="product", hierarchy_path=(),
        source_color=None, color_provenance=None,
        bounds_mm=BoundingBox(min_xyz=(0, 0, z), max_xyz=(size[0], size[1], z + size[2])),
    )


def _xy(import_id: str, offset: float) -> SectionRequest:
    return SectionRequest(import_id=import_id, plane=SectionPlane(kind="XY", offset_mm=offset))


def test_imports_are_evicted_least_recently_used_first() -> None:
    sessions = ImportSessions()
    ids = [sessions.put("a" * 64, ()) for _ in range(MAX_IMPORTS)]
    assert sessions.get(ids[0]) == ()  # touch the oldest
    newest = sessions.put("a" * 64, ())

    assert sessions.get(ids[1]) is None
    assert sessions.get(ids[0]) == () and sessions.get(ids[2]) == ()
    assert sessions.get(newest) == ()


def test_evicting_an_import_drops_its_sections_and_listeners_fire() -> None:
    imports = ImportSessions()
    sections = SectionSessions()
    service = SectionService(imports, sections)
    dropped: list[str] = []
    imports.on_remove(dropped.append)
    first = imports.put("a", (_body("A", (10, 10, 10)),))
    section = service.create(_xy(first, 5))
    for _ in range(MAX_IMPORTS):
        imports.put("b", ())

    assert dropped[0] == first
    assert sections.get(section.section_id) is None
    assert sections.get_response(section.section_id) is None
    with pytest.raises(UnknownImport):
        service.create(_xy(first, 5))


def test_sections_are_bounded_lru_and_get_refreshes() -> None:
    sections = SectionSessions()
    ids = [sections.put({}, "i") for _ in range(MAX_SECTIONS)]
    assert sections.get(ids[0]) == {}
    sections.put({}, "i")

    assert sections.get(ids[1]) is None
    assert sections.get(ids[0]) == {}


def test_misses_plane_only_when_box_is_fully_on_one_side() -> None:
    box = BoundingBox(min_xyz=(0, 0, 0), max_xyz=(1, 1, 1))
    z = (0.0, 0.0, 1.0)
    assert misses_plane(box, (0, 0, 2), z, 1e-6)
    assert misses_plane(box, (0, 0, -1), z, 1e-6)
    assert not misses_plane(box, (0, 0, 1), z, 1e-6)  # touching face
    assert not misses_plane(box, (0, 0, 0.5), z, 1e-6)


def test_prefilter_skips_missed_bodies_without_changing_result(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    imports = ImportSessions()
    bodies = (_body("Low", (10, 10, 10)), _body("High", (10, 10, 10), z=50))
    import_id = imports.put("abc", bodies)
    calls: list[Any] = []

    def spy(shape: Any, plane: SectionPlane) -> Any:
        calls.append(shape)
        return section_shape(shape, plane)

    monkeypatch.setattr("cad2maxwell_backend.services.section_service.section_shape", spy)
    filtered = SectionService(imports, SectionSessions()).create(_xy(import_id, 5))

    assert calls == [bodies[0].shape]
    plane = SectionPlane(kind="XY", offset_mm=5)
    assert section_shape(bodies[1].shape, plane) == ()  # skipped body had no section anyway
    expected = [w.dto.model_copy(update={"role": "outer"})
                for w in section_shape(bodies[0].shape, plane)]
    assert len(filtered.components) == 1
    assert filtered.components[0].wires == expected


def test_identical_request_hits_cache_and_plane_change_misses(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    imports = ImportSessions()
    import_id = imports.put("abc", (_body("A", (10, 10, 10)),))
    service = SectionService(imports, SectionSessions())
    first = service.create(_xy(import_id, 5))
    calls: list[int] = []

    def counting(shape: Any, plane: SectionPlane) -> Any:
        calls.append(1)
        return section_shape(shape, plane)

    monkeypatch.setattr("cad2maxwell_backend.services.section_service.section_shape", counting)

    again = service.create(_xy(import_id, 5 + 1e-8))
    assert calls == []
    other = service.create(_xy(import_id, 6))

    assert again == first
    assert calls == [1]
    assert other.section_id != first.section_id


def test_cache_entry_is_evicted_with_its_section() -> None:
    imports = ImportSessions()
    sections = SectionSessions(max_sections=2)
    service = SectionService(imports, sections)
    import_id = imports.put("abc", (_body("A", (10, 10, 10)),))
    first = service.create(_xy(import_id, 1))
    service.create(_xy(import_id, 2))
    service.create(_xy(import_id, 3))

    assert service.create(_xy(import_id, 1)).section_id != first.section_id


def test_thin_and_tiny_regions_warn_without_removal() -> None:
    imports = ImportSessions()
    import_id = imports.put("abc", (
        _body("Strip", (100, 0.01, 10)), _body("Speck", (0.005, 0.005, 10)),
        _body("Plate", (10, 10, 10)),
    ))
    response = SectionService(imports, SectionSessions()).create(_xy(import_id, 5))
    codes = {(d.code, d.component_id): d for d in response.diagnostics}
    strip, speck, plate = (c.component_id for c in response.components)

    assert codes[("sliver_section_region", strip)].severity == "warning"
    assert ("small_section_region", speck) in codes
    assert not any(d.component_id == plate and d.code.endswith("_section_region")
                   for d in response.diagnostics)
    assert len(response.components) == 3
