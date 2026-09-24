import math

from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.TopoDS import TopoDS_Shape
from pydantic import ValidationError

from cad2maxwell_backend.models.imports import BoundingBox


def shape_bounds_mm(shape: TopoDS_Shape) -> BoundingBox | None:
    """Return finite axis-aligned bounds for a shape already in millimetres."""
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    if box.IsVoid():
        return None
    coordinates = box.Get()
    if not all(math.isfinite(value) for value in coordinates):
        return None
    try:
        return BoundingBox(
            min_xyz=(coordinates[0], coordinates[1], coordinates[2]),
            max_xyz=(coordinates[3], coordinates[4], coordinates[5]),
        )
    except ValidationError:
        return None
