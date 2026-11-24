from pathlib import Path

from OCP.IFSelect import IFSelect_ReturnStatus
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TColStd import TColStd_SequenceOfAsciiString

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


class StepImportError(ModelImportError):
    """The STEP source contains no usable exact solids or cannot be read."""


def _source_unit(
    reader: STEPCAFControl_Reader,
) -> tuple[str | None, float | None, list[ImportDiagnostic]]:
    lengths = TColStd_SequenceOfAsciiString()
    angles = TColStd_SequenceOfAsciiString()
    solids = TColStd_SequenceOfAsciiString()
    reader.Reader().FileUnits(lengths, angles, solids)
    units = [lengths.Value(index).ToCString() for index in range(1, lengths.Length() + 1)]
    normalized = {unit.casefold() for unit in units}
    if len(normalized) != 1:
        return None, None, [ImportDiagnostic(
            code="unknown_source_unit", severity="warning",
            message="STEP source has missing or conflicting length units; source scale is unknown.",
        )]
    unit = units[0]
    scale = UNIT_TO_MM.get(unit.casefold())
    if scale is None:
        return unit, None, [ImportDiagnostic(
            code="unknown_source_unit", severity="warning",
            message=f"STEP length unit '{unit}' is not recognized; source scale is unknown.",
        )]
    return unit, scale, []


def read_step(path: Path) -> ImportedModel:
    """Read exact located solids and XCAF metadata in millimetre coordinates."""
    document = new_document()

    reader = STEPCAFControl_Reader()
    reader.SetNameMode(True)
    reader.SetColorMode(True)
    reader.SetLayerMode(True)
    reader.SetMatMode(True)
    if reader.ReadFile(str(path)) != IFSelect_ReturnStatus.IFSelect_RetDone:
        raise StepImportError("OpenCascade rejected STEP content")
    source_unit, scale, diagnostics = _source_unit(reader)
    diagnostics.extend(check_diagnostics(reader.Reader().WS().ModelCheckList(), "reader", "step"))
    if not reader.Transfer(document):
        raise StepImportError("OpenCascade could not transfer STEP content")
    diagnostics.extend(check_diagnostics(
        reader.Reader().WS().TransferReader().LastCheckList(), "transfer", "step",
    ))

    bodies = collect_bodies(document, "step", diagnostics)
    if not bodies:
        raise StepImportError("STEP file contains no importable solids")
    return ImportedModel(
        bodies=tuple(bodies), source_unit=source_unit, to_mm_scale=scale,
        bounds_mm=aggregate_bounds(bodies), diagnostics=tuple(downgrade_failures(diagnostics)),
    )
