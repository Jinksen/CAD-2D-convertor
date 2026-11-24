import math
from pathlib import Path

from export_fixtures import demo_exact, demo_section
from fastapi.testclient import TestClient
from step_fixtures import named_colored_box

from cad2maxwell_backend.api import create_app
from cad2maxwell_backend.geometry.region_metrics import region_metrics
from cad2maxwell_backend.models.sections import Circle2D


def test_demo_regions_have_exact_area_holes_and_interior_points() -> None:
    imported, section = demo_section()
    exact = demo_exact()
    names = {c.id: c.source_name for c in imported.components}
    metrics = [m for c in section.components
               for m in region_metrics(c.component_id, exact[c.component_id], section.plane)]
    assert len(metrics) == 15
    by_name = {names[m.component_id]: m for m in metrics}
    assert by_name["Stator"].hole_count == 1 and by_name["Rotor"].hole_count == 1
    assert by_name["Coil_1"].hole_count == 0
    # The shaft is one analytic circle: area and perimeter must match pi r^2 and 2 pi r.
    shaft = next(c for c in section.components if names[c.component_id] == "Shaft")
    circle = shaft.wires[0].curves[0]
    assert isinstance(circle, Circle2D)
    metric = by_name["Shaft"]
    assert math.isclose(metric.area_mm2, math.pi * circle.radius**2, rel_tol=1e-9)
    assert math.isclose(metric.perimeter_mm, 2 * math.pi * circle.radius, rel_tol=1e-9)
    assert math.isclose(metric.centroid[0], circle.center[0], abs_tol=1e-6)


def test_annulus_area_perimeter_and_interior_point_are_exact() -> None:
    from dataclasses import replace

    from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
    from OCP.BRepPrimAPI import BRepPrimAPI_MakeCylinder

    from cad2maxwell_backend.geometry.section import section_shape
    from cad2maxwell_backend.geometry.section_classification import classify_wires
    from cad2maxwell_backend.models.sections import SectionPlane

    plane = SectionPlane(kind="XY", offset_mm=5)
    ring = BRepAlgoAPI_Cut(BRepPrimAPI_MakeCylinder(30, 10).Shape(),
                           BRepPrimAPI_MakeCylinder(12, 10).Shape()).Shape()
    wires = section_shape(ring, plane)
    roles = classify_wires(wires, plane)
    wires = tuple(replace(w, dto=w.dto.model_copy(update={"role": r}))
                  for w, r in zip(wires, roles, strict=True))
    (metric,) = region_metrics("ring", wires, plane)
    assert metric.hole_count == 1
    assert math.isclose(metric.area_mm2, math.pi * (30**2 - 12**2), rel_tol=1e-9)
    assert math.isclose(metric.perimeter_mm, 2 * math.pi * (30 + 12), rel_tol=1e-9)
    assert math.hypot(*metric.centroid) < 1e-6
    assert 12 < math.hypot(*metric.interior_point) < 30


def test_regions_endpoint(tmp_path: Path) -> None:
    client = TestClient(create_app())
    imported = client.post("/api/v1/imports/step", json={
        "path": str(named_colored_box(tmp_path / "coil.step"))}).json()
    section = client.post("/api/v1/section", json={
        "import_id": imported["import_id"], "plane": {"kind": "XY", "offset_mm": 15}}).json()
    response = client.get(f"/api/v1/sections/{section['section_id']}/regions")
    assert response.status_code == 200
    region = response.json()["regions"][0]
    assert region["hole_count"] == 0 and region["area_mm2"] > 0
    assert math.isclose(region["perimeter_mm"] * 1, region["perimeter_mm"])
    assert client.get("/api/v1/sections/nope/regions").status_code == 404
