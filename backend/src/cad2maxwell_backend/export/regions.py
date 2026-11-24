"""Regions (outer wire plus its holes) with their exact metrics, shared by all writers."""
from collections.abc import Mapping
from dataclasses import dataclass

from cad2maxwell_backend.export.plan import DxfExportError, ExportPlan, PlannedBody
from cad2maxwell_backend.geometry.region_metrics import region_metrics
from cad2maxwell_backend.geometry.section import ExactSectionWire, SectionError
from cad2maxwell_backend.models.sections import RegionMetric, SectionPlane, SectionWire


@dataclass(frozen=True, slots=True)
class ExportRegion:
    name: str  # ASCII-safe, unique; the region id in every format
    body: PlannedBody
    metric: RegionMetric
    outer: SectionWire
    holes: tuple[SectionWire, ...]


def build_regions(
    plan: ExportPlan, exact: Mapping[str, tuple[ExactSectionWire, ...]] | None,
    plane: SectionPlane,
) -> list[ExportRegion]:
    if exact is None:
        raise DxfExportError("This format needs the exact section; compute the section again")
    regions: list[ExportRegion] = []
    for body in plan.bodies:
        wires = exact.get(body.source.id)
        if wires is None:
            raise DxfExportError("The exact section of a component is no longer available")
        try:
            metrics = region_metrics(body.source.id, wires, plane)
        except SectionError as exc:
            raise DxfExportError(str(exc)) from exc
        for metric in metrics:
            name = (body.source.export_name if len(metrics) == 1
                    else f"{body.source.export_name}_{metric.region_index + 1:02d}")
            dto = body.component.wires
            regions.append(ExportRegion(
                name=name, body=body, metric=metric, outer=dto[metric.outer_wire],
                holes=tuple(dto[i] for i in metric.hole_wires),
            ))
    if len({region.name for region in regions}) != len(regions):
        raise DxfExportError("Region names are not unique")
    return regions
