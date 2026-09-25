import json
import math
from pathlib import Path

import ezdxf
import pytest
from step_fixtures import named_colored_box

from cad2maxwell_backend.export.dxf import DxfExportError, export_dxf
from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.models.imports import ImportResponse
from cad2maxwell_backend.models.sections import (
    Ellipse2D,
    SectionPlane,
    SectionRequest,
    SectionResponse,
)
from cad2maxwell_backend.services.import_service import ImportService
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.section_service import SectionService
from cad2maxwell_backend.services.section_sessions import SectionSessions


def _box_section(tmp_path: Path) -> tuple[ImportResponse, SectionResponse]:
    imports = ImportService(read_step, ImportSessions())
    imported = imports.import_path(str(named_colored_box(tmp_path / "coil.step")))
    section = SectionService(imports.sessions, SectionSessions()).create(SectionRequest(
        import_id=imported.import_id, plane=SectionPlane(kind="XY", offset_mm=15),
    ))
    return imported, section


def test_export_keeps_units_component_layer_and_traceability(tmp_path: Path) -> None:
    imported, section = _box_section(tmp_path)
    target = tmp_path / "coil.dxf"

    result = export_dxf(imported, section, target)

    drawing = ezdxf.readfile(target)
    assert drawing.header["$INSUNITS"] == 4
    assert len(drawing.modelspace().query("LINE")) == 4
    assert {entity.dxf.layer for entity in drawing.modelspace()} == {"Civka"}
    manifest = json.loads(result.manifest_path.read_text(encoding="utf-8"))
    assert manifest["schema_version"] == 1
    assert manifest["validation_status"] == "draft_unvalidated"
    assert manifest["components"][0]["source_name"] == imported.components[0].source_name
    assert manifest["components"][0]["export_name"] == "Civka"
    assert manifest["components"][0]["wires"][0]["role"] == "outer"


def test_export_refuses_overwrite(tmp_path: Path) -> None:
    imported, section = _box_section(tmp_path)
    target = tmp_path / "coil.dxf"
    target.write_text("keep this", encoding="utf-8")

    with pytest.raises(DxfExportError):
        export_dxf(imported, section, target)
    assert target.read_text(encoding="utf-8") == "keep this"


def test_reversed_ellipse_arc_keeps_its_endpoints(tmp_path: Path) -> None:
    imported, section = _box_section(tmp_path)
    section.components[0].wires[0].curves = [Ellipse2D(
        center=(2, 3), major_radius=10, minor_radius=5,
        x_axis=(1, 0), y_axis=(0, -1),
        start_parameter=0.3, end_parameter=0.1,
    )]
    target = tmp_path / "ellipse.dxf"

    export_dxf(imported, section, target)

    ellipse = ezdxf.readfile(target).modelspace().query("ELLIPSE")[0]
    expected = [(2 + 10 * math.cos(t), 3 - 5 * math.sin(t)) for t in (0.1, 0.3)]
    actual = [(ellipse.start_point.x, ellipse.start_point.y),
              (ellipse.end_point.x, ellipse.end_point.y)]
    for got, want in zip(sorted(actual), sorted(expected), strict=True):
        assert got == pytest.approx(want)
