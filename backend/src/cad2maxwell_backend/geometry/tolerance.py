from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class TolerancePolicy:
    linear_mm: float = 1e-6
    join_mm: float = 1e-5
    tiny_edge_mm: float = 1e-4
    area_mm2: float = 1e-8


DEFAULT_TOLERANCE = TolerancePolicy()
