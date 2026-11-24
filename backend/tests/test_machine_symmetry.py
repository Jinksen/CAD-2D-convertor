import math
from pathlib import Path

import pytest
from fastapi.testclient import TestClient
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder
from OCP.gp import gp_Ax1, gp_Dir, gp_Pnt, gp_Trsf
from OCP.TopoDS import TopoDS_Shape

from cad2maxwell_backend.api import create_app
from cad2maxwell_backend.domain.import_result import ImportedBody
from cad2maxwell_backend.geometry.bounds import shape_bounds_mm
from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.machine.models import SectorRequest
from cad2maxwell_backend.machine.service import create_sector, symmetry
from cad2maxwell_backend.models.sections import SectionPlane, SectionRequest
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.section_service import SectionService
from cad2maxwell_backend.services.section_sessions import SectionSessions

DEMO = Path(__file__).resolve().parents[2] / "docs" / "demo" / "demo-motor.step"


def _body(shape: TopoDS_Shape, index: int) -> ImportedBody:
    bounds = shape_bounds_mm(shape)
    assert bounds is not None
    return ImportedBody(
        shape=shape, occurrence_key=str(index), solid_ordinal=0, source_id=str(index),
        source_name=f"B{index}", name_provenance="product", hierarchy_path=(f"B{index}",),
        source_color=None, color_provenance=None, bounds_mm=bounds,
    )


def _service(bodies: list[ImportedBody]) -> tuple[SectionService, str]:
    imports = ImportSessions()
    import_id = imports.put("sym", tuple(bodies))
    service = SectionService(imports, SectionSessions())
    section = service.create(SectionRequest(
        import_id=import_id, plane=SectionPlane(kind="XY", offset_mm=1),
    ))
    return service, section.section_id


def _ring(n: int, with_hub: bool = True) -> list[ImportedBody]:
    """A hub plus n boxes around the z axis (a synthetic n-fold symmetric part)."""
    bodies = [_body(BRepPrimAPI_MakeCylinder(5, 4).Shape(), 0)] if with_hub else []
    for k in range(n):
        box = BRepPrimAPI_MakeBox(gp_Pnt(10, -1, 0), 4, 2, 4).Shape()
        trsf = gp_Trsf()
        trsf.SetRotation(gp_Ax1(gp_Pnt(0, 0, 0), gp_Dir(0, 0, 1)), 2 * math.pi * k / n)
        bodies.append(_body(BRepBuilderAPI_Transform(box, trsf, True).Shape(), k + 1))
    return bodies


@pytest.mark.parametrize("n", [3, 6, 12])
def test_synthetic_ring_symmetry_order(n: int) -> None:
    service, section_id = _service(_ring(n))

    result = symmetry(service, section_id)

    assert result.order == n
    assert result.sector_angle_deg == pytest.approx(360 / n)
    assert result.center == pytest.approx((0, 0), abs=1e-6)
    assert result.confidence == 1.0


def test_asymmetric_section_reports_order_one() -> None:
    bodies = _ring(6)[:-1]  # one box missing: only the hub and 5 of 6

    result = symmetry(*_service(bodies))

    assert result.order == 1 and result.confidence == 0.0


def test_sector_cut_is_exact_and_only_on_request() -> None:
    service, section_id = _service(_ring(6))
    source = service.sections.get_response(section_id)
    assert source is not None

    sector = create_sector(service, section_id, SectorRequest(order=6))

    assert sector.section_id != section_id
    # The hub is cut to a 60 degree wedge (two radial lines and one analytic arc); one box
    # is cut at its boundary-free centre line, the rest are dropped.
    hub = next(c for c in sector.components if c.component_id == source.components[0].component_id)
    kinds = sorted(curve.type for curve in hub.wires[0].curves)
    assert kinds == ["circle", "line", "line"]
    assert all(wire.closed and wire.role for c in sector.components for wire in c.wires)
    assert len(service.sections.get_response(section_id).components) == len(source.components)  # type: ignore[union-attr]


def test_demo_motor_has_fourfold_symmetry_and_sector_api() -> None:
    model = read_step(DEMO)
    app = create_app()
    client = TestClient(app)
    imports = app.state.section_service._imports
    import_id = imports.put("demo", model.bodies)
    section = client.post("/api/v1/section", json={
        "import_id": import_id, "plane": {"kind": "XY", "offset_mm": 25},
    }).json()

    found = client.get(f"/api/v1/sections/{section['section_id']}/symmetry").json()
    assert found["order"] == 4
    assert found["sector_angle_deg"] == 90.0
    assert found["confidence"] == 1.0

    cut = client.post(f"/api/v1/sections/{section['section_id']}/sector", json={"order": 4})
    assert cut.status_code == 200
    assert len(cut.json()["components"]) > 0

    def area(section_id: str) -> float:
        regions = client.get(f"/api/v1/sections/{section_id}/regions").json()["regions"]
        return sum(region["area_mm2"] for region in regions)

    quarter = area(cut.json()["section_id"])
    assert quarter == pytest.approx(area(section["section_id"]) / 4, rel=1e-6)
    assert client.get("/api/v1/sections/nope/symmetry").status_code == 404
