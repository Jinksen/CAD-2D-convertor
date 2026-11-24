from pathlib import Path

from OCP.IFSelect import IFSelect_ReturnStatus
from OCP.IGESCAFControl import IGESCAFControl_Reader

from cad2maxwell_backend.domain.import_result import ImportedModel
from cad2maxwell_backend.geometry.xcaf_model import (
    UNIT_TO_MM,
    ModelImportError,
    aggregate_bounds,
    check_diagnostics,
    collect_bodies,
    downgrade_failures,
    new_document,
)
from cad2maxwell_backend.models.imports import ImportDiagnostic


def _source_unit(reader: IGESCAFControl_Reader) -> tuple[str | None, float | None,
                                                         list[ImportDiagnostic]]:
    section = reader.IGESModel().GlobalSection()
    name = section.UnitName().ToCString() if section.UnitName() is not None else ""
    scale = UNIT_TO_MM.get(name.casefold())
    if scale is None:
        return name or None, None, [ImportDiagnostic(
            code="unknown_source_unit", severity="warning",
            message=f"IGES length unit '{name}' is not recognized; source scale is unknown.",
        )]
    return name, scale, []


def read_iges(path: Path) -> ImportedModel:
    """Read exact IGES solids through XCAF in millimetre coordinates (OCCT converts units)."""
    document = new_document()
    reader = IGESCAFControl_Reader()
    reader.SetNameMode(True)
    reader.SetColorMode(True)
    reader.SetLayerMode(True)
    if reader.ReadFile(str(path)) != IFSelect_ReturnStatus.IFSelect_RetDone:
        raise ModelImportError("OpenCascade rejected IGES content")
    source_unit, scale, diagnostics = _source_unit(reader)
    diagnostics.extend(check_diagnostics(reader.WS().ModelCheckList(), "reader", "iges"))
    if not reader.Transfer(document):
        raise ModelImportError("OpenCascade could not transfer IGES content")
    diagnostics.extend(check_diagnostics(
        reader.WS().TransferReader().LastCheckList(), "transfer", "iges",
    ))
    bodies = collect_bodies(document, "iges", diagnostics)
    if not bodies:
        raise ModelImportError("IGES file contains no importable solids")
    return ImportedModel(
        bodies=tuple(bodies), source_unit=source_unit, to_mm_scale=scale,
        bounds_mm=aggregate_bounds(bodies), diagnostics=tuple(downgrade_failures(diagnostics)),
    )
