import math

from cad2maxwell_backend.machine.models import SymmetryResponse
from cad2maxwell_backend.models.sections import Point2, RegionMetric

MAX_ORDER = 64
AREA_TOLERANCE = 5e-3  # relative
POSITION_TOLERANCE = 1e-3  # relative to the largest radial distance


def symmetry_center(regions: list[RegionMetric]) -> Point2:
    """Area-weighted centroid of the regions; the rotation centre of any symmetric section."""
    total = sum(region.area_mm2 for region in regions)
    return (sum(r.area_mm2 * r.centroid[0] for r in regions) / total,
            sum(r.area_mm2 * r.centroid[1] for r in regions) / total)


def _matched_fraction(
    regions: list[RegionMetric], center: Point2, order: int, tolerance: float,
) -> float:
    angle = 2 * math.pi / order
    cos, sin = math.cos(angle), math.sin(angle)
    free = set(range(len(regions)))
    matched = 0
    for region in regions:
        dx, dy = region.centroid[0] - center[0], region.centroid[1] - center[1]
        target = (center[0] + dx * cos - dy * sin, center[1] + dx * sin + dy * cos)
        for index in sorted(free):
            other = regions[index]
            if (abs(other.area_mm2 - region.area_mm2) <= AREA_TOLERANCE * region.area_mm2
                    and math.dist(other.centroid, target) <= tolerance
                    and other.hole_count == region.hole_count):
                free.discard(index)
                matched += 1
                break
    return matched / len(regions)


def detect_symmetry(regions: list[RegionMetric], max_order: int = MAX_ORDER) -> SymmetryResponse:
    """Largest rotation order N whose 2*pi/N turn maps every region onto a region of equal
    area and hole count. Compares region centroids, not outlines: a necessary condition, so a
    high confidence is a strong hint and the sector cut stays an explicit user action."""
    if not regions:
        return SymmetryResponse(order=1, sector_angle_deg=360.0, center=(0.0, 0.0), confidence=0.0)
    center = symmetry_center(regions)
    reach = max(math.dist(r.centroid, center) for r in regions)
    tolerance = POSITION_TOLERANCE * max(reach, 1.0)
    for order in range(max_order, 1, -1):
        confidence = _matched_fraction(regions, center, order, tolerance)
        if confidence == 1.0:
            return SymmetryResponse(
                order=order, sector_angle_deg=360.0 / order, center=center, confidence=confidence,
            )
    return SymmetryResponse(order=1, sector_angle_deg=360.0, center=center, confidence=0.0)
