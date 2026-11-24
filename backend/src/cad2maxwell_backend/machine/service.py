import math

from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib

from cad2maxwell_backend.geometry.section import SectionError
from cad2maxwell_backend.machine.models import SectorRequest, SymmetryResponse
from cad2maxwell_backend.machine.sector import cut_sector
from cad2maxwell_backend.machine.symmetry import detect_symmetry
from cad2maxwell_backend.models.sections import SectionComponent, SectionResponse
from cad2maxwell_backend.services.section_service import SectionService, UnknownSection


def symmetry(sections: SectionService, section_id: str) -> SymmetryResponse:
    return detect_symmetry(sections.regions(section_id).regions)


def create_sector(
    sections: SectionService, section_id: str, request: SectorRequest,
) -> SectionResponse:
    """Explicit user action: a new section holding only the 1/N sector of the source section."""
    exact = sections.sections.get(section_id)
    source = sections.sections.get_response(section_id)
    if exact is None or source is None:
        raise UnknownSection
    center = request.center or detect_symmetry(sections.regions(section_id).regions).center
    box = Bnd_Box()
    for wires in exact.values():
        for wire in wires:
            BRepBndLib.Add_s(wire.shape, box)
    reach = 2 * box.CornerMin().Distance(box.CornerMax()) + math.hypot(*center)
    cut = cut_sector(exact, source.plane, center, request.order, request.start_angle_deg, reach)
    if not cut:
        raise SectionError("The sector contains no section geometry")
    new_id = sections.sections.put(cut, source.import_id)
    response = SectionResponse(
        section_id=new_id, import_id=source.import_id, plane=source.plane,
        components=[SectionComponent(component_id=cid, wires=[w.dto for w in wires])
                    for cid, wires in cut.items()],
        diagnostics=[], tolerance=source.tolerance,
    )
    sections.sections.keep_response(response)
    return response
