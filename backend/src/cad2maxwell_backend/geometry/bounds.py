import itertools
import math

from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.Precision import Precision
from OCP.TopoDS import TopoDS_Shape
from pydantic import ValidationError

from cad2maxwell_backend.models.imports import BoundingBox
from cad2maxwell_backend.models.sections import Point3


def misses_plane(bounds: BoundingBox, origin: Point3, normal: Point3, margin_mm: float) -> bool:
    """True when the whole box lies strictly on one side of the plane (beyond the margin)."""
    distances = [
        sum((corner[axis] - origin[axis]) * normal[axis] for axis in range(3))
        for corner in itertools.product(*zip(bounds.min_xyz, bounds.max_xyz, strict=True))
    ]
    return min(distances) > margin_mm or max(distances) < -margin_mm


def shape_bounds_mm(shape: TopoDS_Shape) -> BoundingBox | None:
    """Return finite axis-aligned bounds for a shape already in millimetres."""
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    if box.IsVoid() or box.IsOpen():
        return None
    coordinates = box.Get()
    if not all(
        math.isfinite(value) and not Precision.IsInfinite_s(value)
        for value in coordinates
    ):
        return None
    try:
        return BoundingBox(
            min_xyz=(coordinates[0], coordinates[1], coordinates[2]),
            max_xyz=(coordinates[3], coordinates[4], coordinates[5]),
        )
    except ValidationError:
        return None
