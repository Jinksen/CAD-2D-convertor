import pytest
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakePolygon
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox
from OCP.gp import gp_Pnt

from cad2maxwell_backend.domain.import_result import ImportedBody
from cad2maxwell_backend.geometry.section import ExactSectionWire
from cad2maxwell_backend.models.imports import BoundingBox
from cad2maxwell_backend.models.sections import SectionPlane, SectionRequest, SectionWire
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.section_service import SectionService
from cad2maxwell_backend.services.section_sessions import SectionSessions


def test_invalid_face_becomes_component_diagnostic(monkeypatch: pytest.MonkeyPatch) -> None:
    polygon = BRepBuilderAPI_MakePolygon()
    for x, y in ((0, 0), (10, 10), (0, 10), (10, 0), (0, 0)):
        polygon.Add(gp_Pnt(x, y, 0))
    invalid = ExactSectionWire(shape=polygon.Wire(), dto=SectionWire(closed=True, curves=[]))
    monkeypatch.setattr(
        "cad2maxwell_backend.services.section_service.section_shape",
        lambda _shape, _plane: (invalid,),
    )
    body = ImportedBody(
        shape=BRepPrimAPI_MakeBox(10, 10, 20).Shape(), occurrence_key="block",
        solid_ordinal=0, source_id=None, source_name="Block", name_provenance="product",
        hierarchy_path=(), source_color=None, color_provenance=None,
        bounds_mm=BoundingBox(min_xyz=(0, 0, 0), max_xyz=(10, 10, 20)),
    )
    imports = ImportSessions()
    import_id = imports.put("abc", (body,))

    response = SectionService(imports, SectionSessions()).create(SectionRequest(
        import_id=import_id, plane=SectionPlane(kind="XY", offset_mm=0),
    ))

    assert response.diagnostics[0].code == "invalid_section_topology"
    assert response.diagnostics[0].severity == "error"
    assert response.diagnostics[0].component_id == response.components[0].component_id
    assert response.components[0].wires[0].role is None
