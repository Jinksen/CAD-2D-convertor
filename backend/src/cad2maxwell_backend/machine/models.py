from pydantic import Field

from cad2maxwell_backend.models.sections import FiniteModel, Point2


class SymmetryResponse(FiniteModel):
    """order 1 means no rotational symmetry was found (confidence 0)."""

    order: int = Field(ge=1)
    sector_angle_deg: float
    center: Point2
    confidence: float = Field(ge=0, le=1)


class SectorRequest(FiniteModel):
    order: int = Field(ge=2, le=256)
    center: Point2 | None = None  # defaults to the detected symmetry centre
    start_angle_deg: float = 0.0
