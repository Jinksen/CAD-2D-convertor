import pytest
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakePolygon
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder
from OCP.gp import gp_Pnt

from cad2maxwell_backend.geometry.section import ExactSectionWire, SectionError, section_shape
from cad2maxwell_backend.geometry.section_classification import classify_wires
from cad2maxwell_backend.models.sections import SectionPlane, SectionWire


def test_hollow_cylinder_has_one_outer_and_one_hole() -> None:
    cut = BRepAlgoAPI_Cut(
        BRepPrimAPI_MakeCylinder(10, 20).Shape(),
        BRepPrimAPI_MakeCylinder(5, 20).Shape(),
    )
    cut.Build()
    plane = SectionPlane(kind="XY", offset_mm=10)

    roles = classify_wires(section_shape(cut.Shape(), plane), plane)

    assert sorted(roles) == ["hole", "outer"]


def test_simple_box_has_one_outer_wire() -> None:
    plane = SectionPlane(kind="XZ", offset_mm=10)
    roles = classify_wires(section_shape(BRepPrimAPI_MakeBox(10, 20, 30).Shape(), plane), plane)
    assert roles == ("outer",)


def test_self_intersecting_wire_is_rejected() -> None:
    polygon = BRepBuilderAPI_MakePolygon()
    for x, y in ((0, 0), (10, 10), (0, 10), (10, 0), (0, 0)):
        polygon.Add(gp_Pnt(x, y, 0))
    wire = ExactSectionWire(shape=polygon.Wire(), dto=SectionWire(closed=True, curves=[]))

    with pytest.raises(SectionError, match="self-intersects"):
        classify_wires((wire,), SectionPlane(kind="XY", offset_mm=0))
