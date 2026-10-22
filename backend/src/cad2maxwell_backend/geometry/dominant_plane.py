import math
from dataclasses import dataclass

from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepGProp import BRepGProp
from OCP.GeomAbs import GeomAbs_SurfaceType
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_ShapeEnum
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS, TopoDS_Shape

from cad2maxwell_backend.models.sections import Point3

# Faces whose unit normals differ by less than this (about 0.06 degrees) count as parallel.
_PARALLEL_TOLERANCE = 1e-3


class NoPlanarFaceError(ValueError):
    """The body has no planar face to align a section plane with."""


@dataclass(frozen=True, slots=True)
class DominantPlane:
    normal: Point3
    offset_mm: float
    planar_area_mm2: float


def _canonical(normal: Point3) -> Point3:
    # Opposite faces of a sheet share one plane family; pick a stable sign.
    largest = max(range(3), key=lambda index: abs(normal[index]))
    sign = 1.0 if normal[largest] > 0 else -1.0
    return (normal[0] * sign + 0.0, normal[1] * sign + 0.0, normal[2] * sign + 0.0)


def dominant_plane(shape: TopoDS_Shape) -> DominantPlane:
    """Return the plane family with the largest total planar face area.

    For a lamination this is the sheet plane. The offset passes through the
    body's volume centroid, which is mid-thickness for a flat sheet.
    """
    families: list[tuple[Point3, float]] = []
    explorer = TopExp_Explorer(shape, TopAbs_ShapeEnum.TopAbs_FACE)
    while explorer.More():
        face = TopoDS.Face_s(explorer.Current())  # type: ignore[attr-defined]
        surface = BRepAdaptor_Surface(face)
        if surface.GetType() == GeomAbs_SurfaceType.GeomAbs_Plane:
            direction = surface.Plane().Axis().Direction()
            normal = _canonical((direction.X(), direction.Y(), direction.Z()))
            properties = GProp_GProps()
            BRepGProp.SurfaceProperties_s(face, properties)
            for index, (family, area) in enumerate(families):
                if all(abs(a - b) < _PARALLEL_TOLERANCE
                       for a, b in zip(family, normal, strict=True)):
                    families[index] = (family, area + properties.Mass())
                    break
            else:
                families.append((normal, properties.Mass()))
        explorer.Next()
    if not families:
        raise NoPlanarFaceError("The selected body has no planar face.")
    normal, area = max(families, key=lambda family: family[1])
    length = math.sqrt(sum(value * value for value in normal))
    normal = (normal[0] / length, normal[1] / length, normal[2] / length)
    volume = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, volume)
    centroid = volume.CentreOfMass()
    offset = normal[0] * centroid.X() + normal[1] * centroid.Y() + normal[2] * centroid.Z()
    return DominantPlane(normal=normal, offset_mm=offset, planar_area_mm2=area)
