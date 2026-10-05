import math

from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeWire
from OCP.gp import gp_Ax2, gp_Circ, gp_Dir, gp_Pnt

from cad2maxwell_backend.geometry.section import ExactSectionWire
from cad2maxwell_backend.geometry.section_edges import check_section_edges
from cad2maxwell_backend.models.sections import SectionWire


def _line(first: float, last: float) -> ExactSectionWire:
    edge = BRepBuilderAPI_MakeEdge(gp_Pnt(first, 0, 0), gp_Pnt(last, 0, 0)).Edge()
    return ExactSectionWire(
        BRepBuilderAPI_MakeWire(edge).Wire(), SectionWire(closed=False, curves=[]),
    )


def test_tiny_edge_reports_length_and_component_without_changing_geometry() -> None:
    wire = _line(0, 0.00005)
    diagnostics = check_section_edges((wire,), "coil")
    assert len(diagnostics) == 1
    assert diagnostics[0].code == "tiny_section_edge"
    assert diagnostics[0].severity == "warning"
    assert diagnostics[0].component_id == "coil"
    assert "5e-05 mm" in diagnostics[0].message
    assert not wire.shape.IsNull()


def test_independently_created_reversed_edges_are_duplicates() -> None:
    diagnostics = check_section_edges((_line(0, 10), _line(10, 0)), "stator")
    assert [d.code for d in diagnostics] == ["duplicate_section_edge"]
    assert diagnostics[0].severity == "error"
    assert diagnostics[0].component_id == "stator"


def test_complementary_arcs_with_identical_endpoints_are_not_duplicates() -> None:
    circle = gp_Circ(gp_Ax2(gp_Pnt(0, 0, 0), gp_Dir(0, 0, 1)), 10)
    wires = tuple(ExactSectionWire(
        BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(circle, first, last).Edge()).Wire(),
        SectionWire(closed=False, curves=[]),
    ) for first, last in ((0, math.pi), (math.pi, math.tau)))
    assert check_section_edges(wires, "rotor") == []


def test_adjacent_edges_are_not_duplicates() -> None:
    assert check_section_edges((_line(0, 10), _line(10, 20)), "stator") == []


def test_partial_coincident_edges_are_not_full_duplicates() -> None:
    assert check_section_edges((_line(0, 10), _line(0, 5)), "stator") == []


def test_full_circular_edges_are_duplicates() -> None:
    circle = gp_Circ(gp_Ax2(gp_Pnt(0, 0, 0), gp_Dir(0, 0, 1)), 10)
    wires = tuple(ExactSectionWire(
        BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(circle).Edge()).Wire(),
        SectionWire(closed=True, curves=[]),
    ) for _ in range(2))
    assert [d.code for d in check_section_edges(wires, "ring")] == ["duplicate_section_edge"]


def test_short_circular_arc_uses_exact_length() -> None:
    circle = gp_Circ(gp_Ax2(gp_Pnt(0, 0, 0), gp_Dir(0, 0, 1)), 10)
    edge = BRepBuilderAPI_MakeEdge(circle, 0, 0.000005).Edge()
    wire = ExactSectionWire(
        BRepBuilderAPI_MakeWire(edge).Wire(), SectionWire(closed=False, curves=[]),
    )
    diagnostics = check_section_edges((wire,), "arc")
    assert [d.code for d in diagnostics] == ["tiny_section_edge"]
    assert "5e-05 mm" in diagnostics[0].message
