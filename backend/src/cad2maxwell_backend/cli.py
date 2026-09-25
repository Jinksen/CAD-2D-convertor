import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from pydantic import ValidationError

from cad2maxwell_backend.export.dxf import DxfExportError, export_dxf
from cad2maxwell_backend.geometry.section import SectionError
from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.models.sections import SectionPlane, SectionRequest
from cad2maxwell_backend.services.import_service import ImportService, ImportServiceError
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.section_service import SectionService
from cad2maxwell_backend.services.section_sessions import SectionSessions


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Create an unvalidated analytic 2D DXF draft from a local STEP file.",
    )
    parser.add_argument("--input", required=True, help="Absolute STEP/STP input path")
    parser.add_argument("--plane", required=True, choices=("XY", "XZ", "YZ"))
    parser.add_argument("--offset-mm", required=True, type=float)
    parser.add_argument("--output", required=True, help="Absolute .dxf output path")
    args = parser.parse_args(argv)

    try:
        import_service = ImportService(read_step, ImportSessions())
        imported = import_service.import_path(args.input)
        section_service = SectionService(import_service.sessions, SectionSessions())
        section = section_service.create(SectionRequest(
            import_id=imported.import_id,
            plane=SectionPlane(kind=args.plane, offset_mm=args.offset_mm),
        ))
        result = export_dxf(imported, section, Path(args.output))
    except (DxfExportError, ImportServiceError, SectionError, ValidationError) as exc:
        if isinstance(exc, ImportServiceError):
            message = exc.public_message
        elif isinstance(exc, ValidationError):
            message = "Invalid section plane or offset."
        else:
            message = str(exc)
        print(f"Draft export failed: {message}", file=sys.stderr)
        return 2

    print(f"Draft DXF: {result.dxf_path}")
    print(f"Manifest: {result.manifest_path}")
    print(f"Entities: {result.entity_count}")
    if imported.diagnostics:
        print(f"Import diagnostics: {len(imported.diagnostics)} (see manifest)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
