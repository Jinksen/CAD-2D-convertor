from collections.abc import Mapping

from OCP.Bnd import Bnd_Box
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common, BRepAlgoAPI_Cut
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace
from OCP.BRepGProp import BRepGProp
from OCP.gp import gp_Dir, gp_Pln, gp_Pnt
from OCP.GProp import GProp_GProps
from OCP.TopoDS import TopoDS_Shape

from cad2maxwell_backend.geometry.section import ExactSectionWire, SectionError, plane_frame
from cad2maxwell_backend.geometry.tolerance import DEFAULT_TOLERANCE
from cad2maxwell_backend.models.sections import SectionPlane


def _area(shape: TopoDS_Shape) -> float:
    properties = GProp_GProps()
    BRepGProp.SurfaceProperties_s(shape, properties)
    return abs(properties.Mass())


def _bounds(shape: TopoDS_Shape) -> Bnd_Box:
    bounds = Bnd_Box()
    BRepBndLib.Add_s(shape, bounds)
    return bounds


def _faces(wires: tuple[ExactSectionWire, ...], surface: gp_Pln) -> list[TopoDS_Shape]:
    outer: list[TopoDS_Shape] = []
    holes: list[TopoDS_Shape] = []
    for wire in wires:
        if not wire.dto.closed or wire.dto.role is None:
            raise SectionError("Overlap validation requires closed, classified wires")
        builder = BRepBuilderAPI_MakeFace(surface, wire.shape, True)
        if not builder.IsDone():
            raise SectionError("A section wire does not form a planar region")
        (outer if wire.dto.role == "outer" else holes).append(builder.Face())
    regions = []
    for face in outer:
        region = face
        for hole in holes:
            cut = BRepAlgoAPI_Cut(region, hole)
            cut.Build()
            if not cut.IsDone():
                raise SectionError("A section hole could not be subtracted exactly")
            region = cut.Shape()
        regions.append(region)
    return regions


def find_region_overlaps(
    components: Mapping[str, tuple[ExactSectionWire, ...]], plane: SectionPlane,
) -> list[tuple[str, str, float]]:
    """Return positive-area overlaps between distinct component regions in mm²."""
    frame = plane_frame(plane)
    surface = gp_Pln(gp_Pnt(*frame.origin), gp_Dir(*frame.normal))
    regions = {component_id: [(face, _bounds(face)) for face in _faces(wires, surface)]
               for component_id, wires in components.items()}
    identifiers = list(regions)
    overlaps = []
    for index, first_id in enumerate(identifiers):
        for second_id in identifiers[index + 1:]:
            area = 0.0
            for first, first_bounds in regions[first_id]:
                for second, second_bounds in regions[second_id]:
                    if first_bounds.IsOut(second_bounds):
                        continue
                    common = BRepAlgoAPI_Common(first, second)
                    common.Build()
                    if not common.IsDone():
                        raise SectionError("Cross-component overlap could not be checked exactly")
                    area += _area(common.Shape())
            if area > DEFAULT_TOLERANCE.area_mm2:
                overlaps.append((first_id, second_id, area))
    return overlaps
