"""Resolve which layer, colour and group each exported body gets."""
import re
from dataclasses import dataclass

from cad2maxwell_backend.models.exports import ExportGroup, ExportOptions
from cad2maxwell_backend.models.imports import ImportComponent, ImportResponse, Rgb
from cad2maxwell_backend.models.sections import SectionComponent, SectionResponse
from cad2maxwell_backend.naming import assign_export_names


class DxfExportError(ValueError):
    """The section cannot be safely written as this DXF draft."""


GROUP_PALETTE: tuple[Rgb, ...] = (
    (31, 119, 180), (255, 127, 14), (44, 160, 44), (214, 39, 40),
    (148, 103, 189), (140, 86, 75), (227, 119, 194), (23, 190, 207),
)
MATERIAL_COLORS: dict[str, Rgb] = {
    "air": (200, 220, 255), "copper": (184, 115, 51), "aluminum": (190, 190, 200),
    "electrical_steel": (90, 100, 120), "structural_steel": (120, 120, 120),
    "permanent_magnet": (200, 40, 40), "insulation": (230, 200, 90), "custom": (160, 160, 160),
}


@dataclass(frozen=True, slots=True)
class PlannedBody:
    source: ImportComponent
    component: SectionComponent
    layer: str
    color: Rgb | None
    groups: tuple[str, ...]
    primary_group: ExportGroup | None


@dataclass(frozen=True, slots=True)
class ExportPlan:
    options: ExportOptions
    bodies: tuple[PlannedBody, ...]
    group_layers: dict[str, str]  # group name -> ASCII-safe layer/block name


def _color(
    options: ExportOptions, source: ImportComponent, group: ExportGroup | None,
) -> Rgb | None:
    mode = options.color_mode
    if mode == "monochrome":
        return None
    if mode == "group" and group is not None:
        return GROUP_PALETTE[options.groups.index(group) % len(GROUP_PALETTE)]
    if mode == "material" and group is not None and group.material is not None:
        return MATERIAL_COLORS[group.material]
    return source.source_color  # original, or group/material mode without a label


def plan_export(
    imported: ImportResponse, section: SectionResponse, options: ExportOptions,
) -> ExportPlan:
    """Validate the section for export and assign layers; a body in several groups
    takes its first group in request order for layer and colour (all are listed in the
    manifest)."""
    if section.import_id != imported.import_id or not section.components:
        raise DxfExportError("The section is empty or belongs to another import")
    if any(d.severity == "error" for d in section.diagnostics):
        raise DxfExportError("The section has errors and cannot be exported")
    sources = {component.id: component for component in imported.components}
    for group in options.groups:
        if any(member not in sources for member in group.members):
            raise DxfExportError("A group references an unknown component")
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

    memberships = [
        tuple(group for group in options.groups if source.id in group.members)
        for source, _ in selected
    ]
    # Group layer names are deduplicated together with the ungrouped component names.
    group_safe = assign_export_names([group.name for group in options.groups])
    ungrouped = [source.export_name for (source, _), found in zip(selected, memberships,
                 strict=True) if not found]
    resolved = assign_export_names(group_safe + ungrouped) if options.layer_mode == "group" \
        else group_safe
    group_layers = dict(zip((group.name for group in options.groups), resolved, strict=False))
    ungrouped_layers = iter(resolved[len(group_safe):])

    bodies = []
    for (source, component), found in zip(selected, memberships, strict=True):
        primary = found[0] if found else None
        if options.layer_mode == "component" or (
            primary is None and options.layer_mode == "hybrid"
        ):
            layer = source.export_name
        elif primary is None:
            layer = next(ungrouped_layers)
        elif options.layer_mode == "group":
            layer = group_layers[primary.name]
        else:
            layer = f"{group_layers[primary.name]}__{source.export_name}"
        bodies.append(PlannedBody(
            source=source, component=component, layer=layer,
            color=_color(options, source, primary),
            groups=tuple(group.name for group in found), primary_group=primary,
        ))
    if len({body.layer for body in bodies}) != len(bodies) and options.layer_mode != "group":
        raise DxfExportError("Export layer names are not unique")
    return ExportPlan(options, tuple(bodies), group_layers)
