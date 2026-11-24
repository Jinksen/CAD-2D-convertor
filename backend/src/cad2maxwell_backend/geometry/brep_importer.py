from pathlib import Path

from OCP.BRep import BRep_Builder
from OCP.BRepTools import BRepTools
from OCP.TopAbs import TopAbs_ShapeEnum
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS, TopoDS_Shape

from cad2maxwell_backend.domain.import_result import ImportedBody, ImportedModel
from cad2maxwell_backend.geometry.bounds import shape_bounds_mm
from cad2maxwell_backend.geometry.xcaf_model import ModelImportError, aggregate_bounds
from cad2maxwell_backend.models.imports import ImportDiagnostic


def read_brep(path: Path) -> ImportedModel:
    """Read OCCT BREP solids as bodies named `<file stem>_<n>`; the format has no metadata."""
    shape = TopoDS_Shape()
    if not BRepTools.Read_s(shape, str(path), BRep_Builder()):
        raise ModelImportError("OpenCascade rejected BREP content")
    diagnostics = [
        ImportDiagnostic(
            code="brep_no_metadata", severity="info",
            message="BREP carries no names, colours or assembly structure; bodies are named "
                    "from the file name.",
        ),
        ImportDiagnostic(
            code="assumed_source_unit", severity="warning",
            message="BREP has no unit; coordinates are assumed to be millimetres.",
        ),
    ]
    bodies: list[ImportedBody] = []
    solids = TopExp_Explorer(shape, TopAbs_ShapeEnum.TopAbs_SOLID)
    while solids.More():
        solid = TopoDS.Solid(solids.Current())
        bounds = shape_bounds_mm(solid)
        if bounds is None:
            diagnostics.append(ImportDiagnostic(
                code="invalid_body_bounds", severity="warning",
                message="A BREP solid has void or non-finite bounds and was skipped.",
            ))
        else:
            ordinal = len(bodies) + 1
            name = f"{path.stem}_{ordinal}"
            bodies.append(ImportedBody(
                shape=solid, occurrence_key=str(ordinal), solid_ordinal=0,
                source_id=str(ordinal), source_name=name, name_provenance="product",
                hierarchy_path=(name,), source_color=None, color_provenance=None,
                bounds_mm=bounds,
            ))
        solids.Next()
    if not bodies:
        raise ModelImportError("BREP file contains no importable solids")
    return ImportedModel(
        bodies=tuple(bodies), source_unit="mm", to_mm_scale=1.0,
        bounds_mm=aggregate_bounds(bodies), diagnostics=tuple(diagnostics),
    )
