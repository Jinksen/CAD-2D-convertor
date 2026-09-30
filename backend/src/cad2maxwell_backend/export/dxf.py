import json
import math
import re
from dataclasses import dataclass
from pathlib import Path

from ezdxf import colors, units
from ezdxf.filemanagement import new
from ezdxf.layouts.layout import Modelspace

from cad2maxwell_backend.models.imports import ImportComponent, ImportResponse
from cad2maxwell_backend.models.sections import (
    Circle2D,
    Curve2D,
    Ellipse2D,
    Line2D,
    SectionComponent,
    SectionResponse,
)


class DxfExportError(ValueError):
    """The section cannot be safely written as this DXF draft."""


@dataclass(frozen=True, slots=True)
class DxfExportResult:
    dxf_path: Path
    manifest_path: Path
    entity_count: int


def _is_full_span(start: float, end: float) -> bool:
    return math.isclose(abs(end - start), math.tau, abs_tol=1e-9)


def _checked_span(start: float, end: float) -> None:
    if abs(end - start) > math.tau + 1e-9:
        raise DxfExportError("A circular curve spans more than one turn")


def _circle_angle(curve: Circle2D, parameter: float) -> float:
    dx = curve.x_axis[0] * math.cos(parameter) + curve.y_axis[0] * math.sin(parameter)
    dy = curve.x_axis[1] * math.cos(parameter) + curve.y_axis[1] * math.sin(parameter)
    return math.degrees(math.atan2(dy, dx)) % 360


def _ellipse_parameters(curve: Ellipse2D) -> tuple[float, float]:
    _checked_span(curve.start_parameter, curve.end_parameter)
    if curve.minor_radius > curve.major_radius:
        raise DxfExportError("An ellipse has invalid major and minor radii")
    handedness = (curve.x_axis[0] * curve.y_axis[1]
                  - curve.x_axis[1] * curve.y_axis[0])
    if abs(handedness) < 1e-8:
        raise DxfExportError("An ellipse has a degenerate plane basis")
    sign = 1 if handedness > 0 else -1
    first, last = sorted((sign * curve.start_parameter, sign * curve.end_parameter))
    if _is_full_span(first, last):
        return 0, math.tau
    return first % math.tau, last % math.tau


def _add_curve(modelspace: Modelspace, curve: Curve2D, layer: str) -> None:
    attributes = {"layer": layer}
    if isinstance(curve, Line2D):
        modelspace.add_line(curve.start, curve.end, dxfattribs=attributes)
    elif isinstance(curve, Circle2D):
        _checked_span(curve.start_parameter, curve.end_parameter)
        if _is_full_span(curve.start_parameter, curve.end_parameter):
            modelspace.add_circle(curve.center, curve.radius, dxfattribs=attributes)
        else:
            handedness = (curve.x_axis[0] * curve.y_axis[1]
                          - curve.x_axis[1] * curve.y_axis[0])
            if abs(handedness) < 1e-8:
                raise DxfExportError("A circle has a degenerate plane basis")
            first, last = sorted((curve.start_parameter, curve.end_parameter))
            if handedness > 0:
                start_angle, end_angle = _circle_angle(curve, first), _circle_angle(curve, last)
            else:
                start_angle, end_angle = _circle_angle(curve, last), _circle_angle(curve, first)
            modelspace.add_arc(curve.center, curve.radius, start_angle, end_angle,
                               dxfattribs=attributes)
    elif isinstance(curve, Ellipse2D):
        start, end = _ellipse_parameters(curve)
        major_axis = (curve.x_axis[0] * curve.major_radius,
                      curve.x_axis[1] * curve.major_radius)
        modelspace.add_ellipse(
            curve.center, major_axis=major_axis,
            ratio=curve.minor_radius / curve.major_radius,
            start_param=start, end_param=end, dxfattribs=attributes,
        )
    else:
        raise DxfExportError("The section contains an unsupported DXF curve")


def _manifest_component(
    source: ImportComponent, section_component: SectionComponent,
) -> dict[str, object]:
    return {
        "component_id": source.id,
        "source_name": source.source_name,
        "display_name": source.display_name,
        "export_name": source.export_name,
        "hierarchy_path": source.hierarchy_path,
        "source_color": source.source_color,
        "wires": [
            {"role": wire.role, "closed": wire.closed, "curve_count": len(wire.curves)}
            for wire in section_component.wires
        ],
    }


def export_dxf(
    imported: ImportResponse, section: SectionResponse, output_path: Path,
) -> DxfExportResult:
    """Write closed analytic outlines and a traceability manifest; never overwrite."""
    manifest_path = output_path.with_suffix(".json")
    if output_path.suffix.casefold() != ".dxf" or not output_path.is_absolute():
        raise DxfExportError("Provide an absolute .dxf output path")
    if output_path.exists() or manifest_path.exists():
        raise DxfExportError("The DXF or manifest output already exists")
    if not output_path.parent.is_dir():
        raise DxfExportError("The output directory does not exist")
    if section.import_id != imported.import_id or not section.components:
        raise DxfExportError("The section is empty or belongs to another import")
    if any(d.severity == "error" for d in section.diagnostics):
        raise DxfExportError("The section has errors and cannot be exported")

    sources = {component.id: component for component in imported.components}
    selected = []
    for component in section.components:
        source = sources.get(component.component_id)
        if source is None:
            raise DxfExportError("The section references an unknown component")
        if not re.fullmatch(r"[A-Za-z0-9_-]+", source.export_name):
            raise DxfExportError("An export layer name is not ASCII-safe")
        if not component.wires or any(
            not wire.closed or wire.role is None for wire in component.wires
        ):
            raise DxfExportError("Every exported section wire must be closed and classified")
        selected.append((source, component))
    if len({source.export_name for source, _ in selected}) != len(selected):
        raise DxfExportError("Export layer names are not unique")

    drawing = new(dxfversion="R2013")
    drawing.units = units.MM
    modelspace = drawing.modelspace()
    entity_count = 0
    for source, component in selected:
        layer = drawing.layers.new(source.export_name)
        if source.source_color is not None:
            layer.dxf.true_color = colors.rgb2int(source.source_color)
        for wire in component.wires:
            for curve in wire.curves:
                _add_curve(modelspace, curve, source.export_name)
                entity_count += 1
    if drawing.audit().has_errors:
        raise DxfExportError("The generated DXF failed its structural audit")

    manifest = {
        "schema_version": 1,
        "validation_status": "draft_unvalidated",
        "source_file": Path(imported.path).name,
        "source_sha256": imported.sha256,
        "units": "mm",
        "section_plane": section.plane.model_dump(),
        "import_diagnostics": [item.model_dump() for item in imported.diagnostics],
        "section_diagnostics": [item.model_dump() for item in section.diagnostics],
        "components": [_manifest_component(source, component) for source, component in selected],
    }
    created_dxf = False
    created_manifest = False
    try:
        with output_path.open("x", encoding="utf-8", newline="\n") as stream:
            created_dxf = True
            drawing.write(stream)
        with manifest_path.open("x", encoding="utf-8", newline="\n") as stream:
            created_manifest = True
            json.dump(manifest, stream, ensure_ascii=False, indent=2)
    except Exception as exc:
        if created_manifest:
            manifest_path.unlink(missing_ok=True)
        if created_dxf:
            output_path.unlink(missing_ok=True)
        raise DxfExportError("The DXF and manifest could not be written") from exc
    return DxfExportResult(output_path, manifest_path, entity_count)
