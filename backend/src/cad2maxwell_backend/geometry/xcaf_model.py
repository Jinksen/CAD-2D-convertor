"""Format-neutral XCAF helpers shared by the STEP and IGES readers."""
from collections import Counter

from OCP.Interface import Interface_CheckIterator
from OCP.Quantity import Quantity_Color, Quantity_TypeOfColor
from OCP.TCollection import TCollection_ExtendedString
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

from cad2maxwell_backend.domain.import_result import ImportedBody
from cad2maxwell_backend.geometry.bounds import shape_bounds_mm
from cad2maxwell_backend.models.imports import BoundingBox, ImportDiagnostic, Rgb

UNIT_TO_MM = {
    "millimetre": 1.0, "millimeter": 1.0, "mm": 1.0,
    "centimetre": 10.0, "centimeter": 10.0, "cm": 10.0,
    "metre": 1000.0, "meter": 1000.0, "m": 1000.0,
    "inch": 25.4, "in": 25.4,
    "foot": 304.8, "ft": 304.8,
    "mil": 0.0254, "micron": 0.001, "um": 0.001,
}


_MAX_DETAIL_KINDS = 5
_MAX_DETAIL_CHARS = 120


def _ascii_detail(text: str) -> str:
    cleaned = " ".join(text.encode("ascii", "replace").decode("ascii").split())
    return cleaned[:_MAX_DETAIL_CHARS]


def _detail_summary(counts: Counter[str]) -> str:
    shown = [f"{text} (x{count})" for text, count in counts.most_common(_MAX_DETAIL_KINDS)]
    hidden = len(counts) - len(shown)
    return "; ".join(shown) + (f"; and {hidden} more kind(s)" if hidden else "")


class ModelImportError(ValueError):
    """A source file contains no usable exact solids or cannot be read."""


def check_diagnostics(
    checks: Interface_CheckIterator, phase: str, fmt: str,
) -> list[ImportDiagnostic]:
    """Summarize OCCT checks; failures list up to 5 distinct reader messages with counts."""
    warnings = 0
    failures = 0
    details: Counter[str] = Counter()
    checks.Start()
    while checks.More():
        check = checks.Value()
        warnings += check.NbWarnings()
        failures += check.NbFails()
        details.update(
            _ascii_detail(check.CFail(index)) for index in range(1, check.NbFails() + 1)
        )
        checks.Next()
    diagnostics = []
    if warnings:
        diagnostics.append(ImportDiagnostic(
            code=f"{fmt}_{phase}_warning", severity="warning",
            message=f"OpenCascade reported {warnings} {fmt.upper()} {phase} warning(s).",
        ))
    if failures:
        message = f"OpenCascade reported {failures} {fmt.upper()} {phase} failure(s)."
        if details:
            message += f" Messages: {_detail_summary(details)}."
        diagnostics.append(ImportDiagnostic(
            code=f"{fmt}_{phase}_failure", severity="error", message=message,
        ))
    return diagnostics


def downgrade_failures(diagnostics: list[ImportDiagnostic]) -> list[ImportDiagnostic]:
    """Reader failures are warnings once bodies were imported; the body list stays authoritative."""
    return [
        diagnostic.model_copy(update={"severity": "warning"})
        if diagnostic.code.endswith("_failure") and diagnostic.severity == "error" else diagnostic
        for diagnostic in diagnostics
    ]


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


def aggregate_bounds(bodies: list[ImportedBody]) -> BoundingBox:
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


def new_document() -> TDocStd_Document:
    document = TDocStd_Document(TCollection_ExtendedString("XmlXCAF"))
    XCAFApp_Application.GetApplication_s().InitDocument(document)
    XCAFDoc_DocumentTool.SetLengthUnit_s(document, 0.001)
    return document


def collect_bodies(
    document: TDocStd_Document, fmt: str, diagnostics: list[ImportDiagnostic],
) -> list[ImportedBody]:
    """Walk the XCAF assembly and return one body per located solid."""
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
                        message=f"A {fmt.upper()} solid has void or non-finite bounds; skipped.",
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
                    message=f"A {fmt.upper()} shape contained no importable solid.",
                ))
        explorer.Next()

    return bodies
