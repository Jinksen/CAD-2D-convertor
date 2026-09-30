import math

from OCP.Bnd import Bnd_Box
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_ShapeEnum
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS, TopoDS_Edge, TopoDS_Shape

from cad2maxwell_backend.geometry.section import ExactSectionWire
from cad2maxwell_backend.geometry.tolerance import DEFAULT_TOLERANCE, TolerancePolicy
from cad2maxwell_backend.models.sections import SectionDiagnostic


def _length(shape: TopoDS_Shape) -> float:
    properties = GProp_GProps()
    BRepGProp.LinearProperties_s(shape, properties)
    return abs(properties.Mass())


def check_section_edges(
    wires: tuple[ExactSectionWire, ...], component_id: str,
    tolerance: TolerancePolicy = DEFAULT_TOLERANCE,
) -> list[SectionDiagnostic]:
    """Inspect exact edges within one component without repairing or deleting them."""
    edges: list[tuple[TopoDS_Edge, float, Bnd_Box]] = []
    diagnostics: list[SectionDiagnostic] = []
    for wire in wires:
        explorer = TopExp_Explorer(wire.shape, TopAbs_ShapeEnum.TopAbs_EDGE)
        while explorer.More():
            edge = TopoDS.Edge(explorer.Current())
            length = _length(edge)
            bounds = Bnd_Box()
            BRepBndLib.Add_s(edge, bounds)
            edges.append((edge, length, bounds))
            explorer.Next()
    tiny = [length for _, length, _ in edges if length < tolerance.tiny_edge_mm]
    if tiny:
        diagnostics.append(SectionDiagnostic(
            code="tiny_section_edge", severity="warning", component_id=component_id,
            message=(f"{len(tiny)} section edge(s) are shorter than "
                     f"{tolerance.tiny_edge_mm:g} mm; shortest is {min(tiny):g} mm. "
                     "Inspect these features before Maxwell import; no edges were removed."),
        ))
    duplicates = 0
    for index, (first, first_length, first_bounds) in enumerate(edges):
        for second, second_length, second_bounds in edges[index + 1:]:
            if first_bounds.IsOut(second_bounds):
                continue
            common = BRepAlgoAPI_Common(first, second)
            if not common.IsDone():
                diagnostics.append(SectionDiagnostic(
                    code="section_edge_check_failed", severity="error", component_id=component_id,
                    message="Exact duplicate-edge validation failed; recompute or inspect the CAD.",
                ))
                return diagnostics
            shared = _length(common.Shape())
            # Relative agreement prevents short, partly overlapping edges being
            # mistaken for full duplicates merely because of the absolute tolerance.
            agreement = min(tolerance.linear_mm, min(first_length, second_length) * 1e-6)
            if shared > 0 and math.isclose(shared, first_length, rel_tol=0, abs_tol=agreement) \
                    and math.isclose(shared, second_length, rel_tol=0, abs_tol=agreement):
                duplicates += 1
    if duplicates:
        diagnostics.append(SectionDiagnostic(
            code="duplicate_section_edge", severity="error", component_id=component_id,
            message=(f"{duplicates} duplicate section edge pair(s) exist within this component. "
                     "Inspect the source geometry; no duplicates were removed."),
        ))
    return diagnostics
