from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace
from OCP.BRepGProp import BRepGProp
from OCP.gp import gp_Dir, gp_Pln, gp_Pnt
from OCP.GProp import GProp_GProps

from cad2maxwell_backend.formatting import format_quantity
from cad2maxwell_backend.geometry.section import ExactSectionWire, plane_frame
from cad2maxwell_backend.geometry.tolerance import DEFAULT_TOLERANCE, TolerancePolicy
from cad2maxwell_backend.models.sections import SectionDiagnostic, SectionPlane


def check_section_regions(
    wires: tuple[ExactSectionWire, ...], plane: SectionPlane, component_id: str, name: str,
    tolerance: TolerancePolicy = DEFAULT_TOLERANCE,
) -> list[SectionDiagnostic]:
    """Warn about tiny or sliver outer regions; nothing is removed or repaired."""
    frame = plane_frame(plane)
    surface = gp_Pln(gp_Pnt(*frame.origin), gp_Dir(*frame.normal))
    small: list[float] = []
    slivers: list[float] = []
    for wire in wires:
        if not wire.dto.closed or wire.dto.role != "outer":
            continue
        builder = BRepBuilderAPI_MakeFace(surface, wire.shape, True)
        if not builder.IsDone():
            continue
        face_props, wire_props = GProp_GProps(), GProp_GProps()
        BRepGProp.SurfaceProperties_s(builder.Face(), face_props)
        BRepGProp.LinearProperties_s(wire.shape, wire_props)
        area, perimeter = abs(face_props.Mass()), abs(wire_props.Mass())
        if area < tolerance.small_region_mm2:
            small.append(area)
        elif perimeter > 0 and area / perimeter**2 < tolerance.sliver_ratio:
            slivers.append(area / perimeter**2)
    diagnostics = []
    if small:
        diagnostics.append(SectionDiagnostic(
            code="small_section_region", severity="warning", component_id=component_id,
            message=(f"{name}: {len(small)} region(s) are smaller than "
                     f"{format_quantity(tolerance.small_region_mm2, 'mm²')}; smallest is "
                     f"{format_quantity(min(small), 'mm²')}. Nothing was removed."),
        ))
    if slivers:
        diagnostics.append(SectionDiagnostic(
            code="sliver_section_region", severity="warning", component_id=component_id,
            message=(f"{name}: {len(slivers)} region(s) are sliver-like "
                     f"(area/perimeter² below {tolerance.sliver_ratio:g}); lowest is "
                     f"{min(slivers):.2g}. Nothing was removed."),
        ))
    return diagnostics
