from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class TolerancePolicy:
    linear_mm: float = 1e-6
    join_mm: float = 1e-5
    tiny_edge_mm: float = 1e-4
    area_mm2: float = 1e-8
    small_region_mm2: float = 1e-4
    # area / perimeter**2: a circle is ~0.08, a 500:1 rectangle is ~5e-4.
    sliver_ratio: float = 5e-4


DEFAULT_TOLERANCE = TolerancePolicy()
