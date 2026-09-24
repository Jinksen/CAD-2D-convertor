from dataclasses import dataclass
from typing import Literal

from cad2maxwell_backend.models.imports import BoundingBox, ImportDiagnostic, Rgb


@dataclass(frozen=True, slots=True)
class ImportedBody:
    """A located exact shape and its source metadata, owned by the backend."""

    shape: object
    occurrence_key: str
    solid_ordinal: int
    source_id: str | None
    source_name: str | None
    name_provenance: Literal["occurrence", "product"] | None
    hierarchy_path: tuple[str, ...]
    source_color: Rgb | None
    color_provenance: Literal["instance", "shape"] | None
    bounds_mm: BoundingBox


@dataclass(frozen=True, slots=True)
class ImportedModel:
    bodies: tuple[ImportedBody, ...]
    source_unit: str | None
    to_mm_scale: float | None
    bounds_mm: BoundingBox
    diagnostics: tuple[ImportDiagnostic, ...]
