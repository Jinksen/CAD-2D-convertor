import json
import math
from dataclasses import dataclass
from pathlib import Path

from ezdxf import colors, units
from ezdxf.filemanagement import new
from ezdxf.layouts.layout import Modelspace

from cad2maxwell_backend.export.plan import DxfExportError, ExportPlan, PlannedBody, plan_export
from cad2maxwell_backend.models.exports import ExportOptions
from cad2maxwell_backend.models.imports import ImportResponse
from cad2maxwell_backend.models.sections import (
    Circle2D,
    Curve2D,
    Ellipse2D,
    Line2D,
    SectionResponse,
)


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


def _add_curve(
    modelspace: Modelspace, curve: Curve2D, layer: str, true_color: int | None = None,
) -> None:
    attributes: dict[str, object] = {"layer": layer}
    if true_color is not None:
        attributes["true_color"] = true_color
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


def _manifest_component(body: PlannedBody) -> dict[str, object]:
    source = body.source
    group = body.primary_group
    return {
        "component_id": source.id,
        "source_name": source.source_name,
        "display_name": source.display_name,
        "export_name": source.export_name,
        "layer": body.layer,
        "hierarchy_path": source.hierarchy_path,
        "source_color": source.source_color,
        "groups": list(body.groups),
        "primary_group": group.name if group else None,
        "material_tag": group.material if group else None,
        "wires": [
            {"role": wire.role, "closed": wire.closed, "curve_count": len(wire.curves)}
            for wire in body.component.wires
        ],
    }


def manifest_for(
    imported: ImportResponse, section: SectionResponse, plan: ExportPlan,
) -> dict[str, object]:
    """Schema 2 manifest (see docs/dxf-export.md); shared by every writer."""
    return {
        "schema_version": 2,
        "validation_status": "draft_unvalidated",
        "source_file": Path(imported.path).name,
        "source_sha256": imported.sha256,
        "units": "mm",
        "layer_mode": plan.options.layer_mode,
        "color_mode": plan.options.color_mode,
        "section_plane": section.plane.model_dump(),
        **({"tolerance": section.tolerance.model_dump(exclude_none=True)}
           if section.tolerance is not None else {}),
        "import_diagnostics": [item.model_dump() for item in imported.diagnostics],
        "section_diagnostics": [item.model_dump() for item in section.diagnostics],
        "groups": [
            {"name": group.name, "layer_name": plan.group_layers[group.name],
             "material": group.material, "material_note": group.material_note,
             "members": group.members}
            for group in plan.options.groups
        ],
        "components": [_manifest_component(body) for body in plan.bodies],
    }


def export_dxf(
    imported: ImportResponse, section: SectionResponse, output_path: Path,
    options: ExportOptions | None = None,
) -> DxfExportResult:
    """Write closed analytic outlines and a traceability manifest; never overwrite."""
    manifest_path = output_path.with_suffix(".json")
    if output_path.suffix.casefold() != ".dxf" or not output_path.is_absolute():
        raise DxfExportError("Provide an absolute .dxf output path")
    if output_path.exists() or manifest_path.exists():
        raise DxfExportError("The DXF or manifest output already exists")
    if not output_path.parent.is_dir():
        raise DxfExportError("The output directory does not exist")
    plan = plan_export(imported, section, options or ExportOptions())

    drawing = new(dxfversion="R2013")
    drawing.units = units.MM
    modelspace = drawing.modelspace()
    entity_count = 0
    for body in plan.bodies:
        if body.layer in drawing.layers:
            layer = drawing.layers.get(body.layer)
        else:
            layer = drawing.layers.new(body.layer)
            if body.color is not None:
                layer.dxf.true_color = colors.rgb2int(body.color)
        # A shared layer carries one colour; bodies that differ override per entity.
        layer_color = layer.dxf.get("true_color")
        override = (colors.rgb2int(body.color) if body.color is not None
                    and colors.rgb2int(body.color) != layer_color else None)
        for wire in body.component.wires:
            for curve in wire.curves:
                _add_curve(modelspace, curve, body.layer, override)
                entity_count += 1
    if drawing.audit().has_errors:
        raise DxfExportError("The generated DXF failed its structural audit")

    manifest = manifest_for(imported, section, plan)
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
