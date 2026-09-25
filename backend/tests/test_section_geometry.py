import math

import pytest
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder
from OCP.gp import gp_Ax1, gp_Dir, gp_Pnt, gp_Trsf
from pydantic import ValidationError

from cad2maxwell_backend.geometry.section import plane_frame, section_shape
from cad2maxwell_backend.models.sections import Line2D, SectionPlane


def test_xy_box_section_is_closed_exact_lines() -> None:
    plane = SectionPlane(kind="XY", offset_mm=15)
    wires = section_shape(BRepPrimAPI_MakeBox(10, 20, 30).Shape(), plane)

    assert len(wires) == 1
    assert wires[0].dto.closed
    assert len(wires[0].dto.curves) == 4
    assert all(curve.type == "line" for curve in wires[0].dto.curves)
    points = [point for curve in wires[0].dto.curves for point in (curve.start, curve.end)]
    assert min(point[0] for point in points) == pytest.approx(0)
    assert max(point[0] for point in points) == pytest.approx(10)
    assert min(point[1] for point in points) == pytest.approx(0)
    assert max(point[1] for point in points) == pytest.approx(20)


def test_plane_outside_box_returns_no_wires() -> None:
    assert section_shape(BRepPrimAPI_MakeBox(10, 20, 30).Shape(),
                         SectionPlane(kind="XY", offset_mm=100)) == ()


def test_oblique_cylinder_section_preserves_ellipse() -> None:
    transform = gp_Trsf()
    transform.SetRotation(gp_Ax1(gp_Pnt(0, 0, 0), gp_Dir(0, 1, 0)), math.pi / 4)
    shape = BRepBuilderAPI_Transform(
        BRepPrimAPI_MakeCylinder(5, 30).Shape(), transform, True,
    ).Shape()
    wires = section_shape(shape, SectionPlane(kind="XY", offset_mm=10))

    assert len(wires) == 1
    assert wires[0].dto.closed
    assert any(curve.type == "ellipse" for curve in wires[0].dto.curves)


def test_hollow_cylinder_keeps_outer_and_inner_closed_wires() -> None:
    hollow = BRepAlgoAPI_Cut(
        BRepPrimAPI_MakeCylinder(10, 20).Shape(),
        BRepPrimAPI_MakeCylinder(5, 20).Shape(),
    )
    hollow.Build()
    wires = section_shape(hollow.Shape(), SectionPlane(kind="XY", offset_mm=10))

    assert len(wires) == 2
    assert all(wire.dto.closed for wire in wires)
    assert all(curve.type == "circle" for wire in wires for curve in wire.dto.curves)


def test_custom_plane_rejects_nonorthogonal_basis() -> None:
    with pytest.raises(ValidationError):
        SectionPlane(kind="custom", origin_xyz=(0, 0, 0),
                     normal_xyz=(0, 0, 1), x_dir_xyz=(1, 0, 1))


def test_section_dto_rejects_nonfinite_coordinates() -> None:
    with pytest.raises(ValidationError):
        Line2D(start=(0, float("nan")), end=(1, 0))


@pytest.mark.parametrize(("kind", "offset", "point", "expected"), [
    ("XY", 5, (2, 3, 5), (2, 3)),
    ("XZ", 3, (2, 3, 5), (2, 5)),
    ("YZ", 2, (2, 3, 5), (3, 5)),
])
def test_standard_plane_basis_is_right_handed(
    kind: str, offset: float, point: tuple[float, float, float],
    expected: tuple[float, float],
) -> None:
    frame = plane_frame(SectionPlane(kind=kind, offset_mm=offset))  # type: ignore[arg-type]
    assert frame.project(gp_Pnt(*point)) == pytest.approx(expected)
    cross = (
        frame.x_dir[1] * frame.y_dir[2] - frame.x_dir[2] * frame.y_dir[1],
        frame.x_dir[2] * frame.y_dir[0] - frame.x_dir[0] * frame.y_dir[2],
        frame.x_dir[0] * frame.y_dir[1] - frame.x_dir[1] * frame.y_dir[0],
    )
    assert cross == pytest.approx(frame.normal)
