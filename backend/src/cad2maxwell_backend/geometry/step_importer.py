from pathlib import Path

from OCP.IFSelect import IFSelect_ReturnStatus
from OCP.Quantity import Quantity_Color, Quantity_TypeOfColor
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TCollection import TCollection_ExtendedString
from OCP.TColStd import TColStd_SequenceOfAsciiString
from OCP.TDataStd import TDataStd_Name
from OCP.TDF import TDF_Label
from OCP.TDocStd import TDocStd_Document
from OCP.TopAbs import TopAbs_ShapeEnum
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.XCAFApp import XCAFApp_Application
from OCP.XCAFDoc import XCAFDoc_ColorTool, XCAFDoc_ColorType, XCAFDoc_DocumentTool
from OCP.XCAFPrs import (
    XCAFPrs_DocumentExplorer,
    XCAFPrs_DocumentExplorerFlags_None,
    XCAFPrs_Style,
)

from cad2maxwell_backend.domain.import_result import ImportedBody, ImportedModel
from cad2maxwell_backend.geometry.bounds import shape_bounds_mm
from cad2maxwell_backend.models.imports import BoundingBox, ImportDiagnostic, Rgb


class StepImportError(ValueError):
    """The STEP source contains no usable exact solids or cannot be read."""


_UNIT_TO_MM = {
    "millimetre": 1.0, "millimeter": 1.0, "mm": 1.0,
    "centimetre": 10.0, "centimeter": 10.0, "cm": 10.0,
    "metre": 1000.0, "meter": 1000.0, "m": 1000.0,
    "inch": 25.4, "in": 25.4,
    "foot": 304.8, "ft": 304.8,
}


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
    scale = _UNIT_TO_MM.get(unit.casefold())
    if scale is None:
        return unit, None, [ImportDiagnostic(
            code="unknown_source_unit", severity="warning",
            message=f"STEP length unit '{unit}' is not recognized; source scale is unknown.",
        )]
    return unit, scale, []


def _label_name(label: TDF_Label) -> str | None:
    attribute = TDataStd_Name()
    if label.FindAttribute(TDataStd_Name.GetID_s(), attribute):
        return attribute.Get().ToExtString().strip() or None
    return None


def _rgb(color: Quantity_Color) -> Rgb:
    values = color.Values(Quantity_TypeOfColor.Quantity_TOC_sRGB)
    return (
        round(max(0.0, min(1.0, values[0])) * 255),
        round(max(0.0, min(1.0, values[1])) * 255),
        round(max(0.0, min(1.0, values[2])) * 255),
    )


def _label_color(color_tool: XCAFDoc_ColorTool, label: TDF_Label) -> Rgb | None:
    for color_type in (XCAFDoc_ColorType.XCAFDoc_ColorSurf, XCAFDoc_ColorType.XCAFDoc_ColorGen):
        color = Quantity_Color()
        if color_tool.GetColor_s(label, color_type, color):
            return _rgb(color)
    return None


def _aggregate_bounds(bodies: list[ImportedBody]) -> BoundingBox:
    return BoundingBox(
        min_xyz=(
            min(body.bounds_mm.min_xyz[0] for body in bodies),
            min(body.bounds_mm.min_xyz[1] for body in bodies),
            min(body.bounds_mm.min_xyz[2] for body in bodies),
        ),
        max_xyz=(
            max(body.bounds_mm.max_xyz[0] for body in bodies),
            max(body.bounds_mm.max_xyz[1] for body in bodies),
            max(body.bounds_mm.max_xyz[2] for body in bodies),
        ),
    )


def read_step(path: Path) -> ImportedModel:
    """Read exact located solids and XCAF metadata in millimetre coordinates."""
    document = TDocStd_Document(TCollection_ExtendedString("XmlXCAF"))
    XCAFApp_Application.GetApplication_s().InitDocument(document)
    XCAFDoc_DocumentTool.SetLengthUnit_s(document, 0.001)

    reader = STEPCAFControl_Reader()
    reader.SetNameMode(True)
    reader.SetColorMode(True)
    reader.SetLayerMode(True)
    reader.SetMatMode(True)
    if reader.ReadFile(str(path)) != IFSelect_ReturnStatus.IFSelect_RetDone:
        raise StepImportError("OpenCascade rejected STEP content")
    source_unit, scale, diagnostics = _source_unit(reader)
    if not reader.Transfer(document):
        raise StepImportError("OpenCascade could not transfer STEP content")

    colors = XCAFDoc_DocumentTool.ColorTool_s(document.Main())
    explorer = XCAFPrs_DocumentExplorer(
        document, XCAFPrs_DocumentExplorerFlags_None, XCAFPrs_Style(),
    )
    hierarchy: list[str] = []
    bodies: list[ImportedBody] = []
    while explorer.More():
        node = explorer.Current()
        depth = explorer.CurrentDepth()
        occurrence_name = _label_name(node.Label)
        product_name = _label_name(node.RefLabel)
        hierarchy[depth:] = [occurrence_name or product_name or ""]
        if not node.IsAssembly:
            located = explorer.FindShapeFromPathId_s(document, node.Id)
            solids = TopExp_Explorer(located, TopAbs_ShapeEnum.TopAbs_SOLID)
            ordinal = 0
            while solids.More():
                solid = TopoDS.Solid(solids.Current())
                bounds = shape_bounds_mm(solid)
                if bounds is None:
                    diagnostics.append(ImportDiagnostic(
                        code="invalid_body_bounds", severity="warning",
                        message="A STEP solid has void or non-finite bounds and was skipped.",
                    ))
                else:
                    instance_color = _label_color(colors, node.Label)
                    shape_color = _label_color(colors, node.RefLabel)
                    color = instance_color or shape_color
                    bodies.append(ImportedBody(
                        shape=solid,
                        occurrence_key=node.Id.ToCString(),
                        solid_ordinal=ordinal,
                        source_id=node.Id.ToCString(),
                        source_name=occurrence_name or product_name,
                        name_provenance=(
                            "occurrence" if occurrence_name else "product" if product_name else None
                        ),
                        hierarchy_path=tuple(hierarchy),
                        source_color=color,
                        color_provenance=(
                            "instance" if instance_color else "shape" if shape_color else None
                        ),
                        bounds_mm=bounds,
                    ))
                ordinal += 1
                solids.Next()
            if ordinal == 0:
                diagnostics.append(ImportDiagnostic(
                    code="skipped_non_solid", severity="warning",
                    message="A STEP shape contained no importable solid.",
                ))
        explorer.Next()

    if not bodies:
        raise StepImportError("STEP file contains no importable solids")
    return ImportedModel(
        bodies=tuple(bodies), source_unit=source_unit, to_mm_scale=scale,
        bounds_mm=_aggregate_bounds(bodies), diagnostics=tuple(diagnostics),
    )
