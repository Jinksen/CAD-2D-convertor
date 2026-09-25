from dataclasses import replace

import pytest
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder
from OCP.gp import gp_Trsf, gp_Vec
from OCP.TopoDS import TopoDS_Shape

from cad2maxwell_backend.geometry.section import ExactSectionWire, section_shape
from cad2maxwell_backend.geometry.section_classification import classify_wires
from cad2maxwell_backend.geometry.section_validation import find_region_overlaps
from cad2maxwell_backend.models.sections import SectionPlane

PLANE = SectionPlane(kind="XY", offset_mm=10)


def _translated(shape: TopoDS_Shape, x: float) -> TopoDS_Shape:
    transform = gp_Trsf()
    transform.SetTranslation(gp_Vec(x, 0, 0))
    return BRepBuilderAPI_Transform(shape, transform, True).Shape()


def _classified(shape: TopoDS_Shape) -> tuple[ExactSectionWire, ...]:
    wires = section_shape(shape, PLANE)
    roles = classify_wires(wires, PLANE)
    return tuple(
        replace(wire, dto=wire.dto.model_copy(update={"role": role}))
        for wire, role in zip(wires, roles, strict=True)
    )


def test_detects_positive_area_overlap_between_components() -> None:
    first = BRepPrimAPI_MakeBox(10, 10, 20).Shape()
    second = _translated(BRepPrimAPI_MakeBox(10, 10, 20).Shape(), 5)

    overlaps = find_region_overlaps({"a": _classified(first), "b": _classified(second)}, PLANE)

    assert overlaps == [("a", "b", pytest.approx(50))]


def test_touching_regions_have_no_area_overlap() -> None:
    first = BRepPrimAPI_MakeBox(10, 10, 20).Shape()
    second = _translated(BRepPrimAPI_MakeBox(10, 10, 20).Shape(), 10)

    assert find_region_overlaps({"a": _classified(first), "b": _classified(second)}, PLANE) == []


def test_region_inside_another_components_hole_is_not_overlap() -> None:
    ring = BRepAlgoAPI_Cut(
        BRepPrimAPI_MakeCylinder(10, 20).Shape(),
        BRepPrimAPI_MakeCylinder(5, 20).Shape(),
    )
    ring.Build()
    center = BRepPrimAPI_MakeCylinder(3, 20).Shape()

    assert find_region_overlaps({"ring": _classified(ring.Shape()),
                                 "center": _classified(center)}, PLANE) == []
