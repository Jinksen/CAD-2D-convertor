from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder

from cad2maxwell_backend.geometry.section import section_shape
from cad2maxwell_backend.geometry.section_classification import classify_wires
from cad2maxwell_backend.models.sections import SectionPlane


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
