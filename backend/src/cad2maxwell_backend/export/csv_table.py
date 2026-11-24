"""Region table: raw millimetre values, deterministic, one row per region."""
import csv
import io

from cad2maxwell_backend.export.primitives import number
from cad2maxwell_backend.export.regions import ExportRegion

HEADER = ["region", "source_body", "groups", "area_mm2", "perimeter_mm",
          "centroid_x_mm", "centroid_y_mm", "holes"]


def write_csv(regions: list[ExportRegion]) -> str:
    out = io.StringIO(newline="")
    writer = csv.writer(out, lineterminator="\n")
    writer.writerow(HEADER)
    for region in regions:
        metric, source = region.metric, region.body.source
        writer.writerow([
            region.name, source.source_name or source.display_name,
            ";".join(region.body.groups), number(metric.area_mm2),
            number(metric.perimeter_mm), number(metric.centroid[0]),
            number(metric.centroid[1]), metric.hole_count,
        ])
    return out.getvalue()
