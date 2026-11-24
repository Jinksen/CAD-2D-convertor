import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from pydantic import ValidationError

from cad2maxwell_backend.batch_job import load_job
from cad2maxwell_backend.export.dxf import DxfExportError, export_dxf
from cad2maxwell_backend.geometry.model_reader import read_model
from cad2maxwell_backend.geometry.section import SectionError
from cad2maxwell_backend.models.sections import SectionPlane, SectionRequest
from cad2maxwell_backend.services.import_service import ImportService, ImportServiceError
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.section_service import SectionService
from cad2maxwell_backend.services.section_sessions import SectionSessions


def run_job(job_path: Path) -> int:
    """Export every job section into the output folder; refuses to overwrite and writes nothing
    unless all targets are free."""
    try:
        job = load_job(job_path)
        output_dir = Path(job.output_dir)
        if not output_dir.is_absolute() or not output_dir.is_dir():
            raise ValueError("output_dir must be an existing absolute folder")
        unsupported = {f for section in job.sections for f in section.formats if f != "dxf"}
        if unsupported:
            raise ValueError(f"export formats not available yet: {', '.join(sorted(unsupported))}")
        targets = [output_dir / f"{section.name}.dxf" for section in job.sections]
        clashes = [t.name for t in targets if t.exists() or t.with_suffix(".json").exists()]
        if clashes:
            raise ValueError(f"outputs already exist: {', '.join(clashes)}")
        import_service = ImportService(read_model, ImportSessions())
        imported = import_service.import_path(job.input)
        section_service = SectionService(import_service.sessions, SectionSessions())
        for section, target in zip(job.sections, targets, strict=True):
            created = section_service.create(SectionRequest(
                import_id=imported.import_id,
                plane=SectionPlane(kind=section.plane, offset_mm=section.offset_mm),
            ))
            result = export_dxf(imported, created, target)
            print(f"{section.name}: {result.dxf_path} ({result.entity_count} entities)")
    except (DxfExportError, ImportServiceError, SectionError, ValidationError, ValueError,
            OSError) as exc:
        message = exc.public_message if isinstance(exc, ImportServiceError) else str(exc)
        print(f"Batch export failed: {message}", file=sys.stderr)
        return 2
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Create an unvalidated analytic 2D DXF draft from a local STEP file.",
    )
    parser.add_argument("--input", help="Absolute STEP/IGES/BREP input path")
    parser.add_argument("--plane", choices=("XY", "XZ", "YZ"))
    parser.add_argument("--offset-mm", type=float)
    parser.add_argument("--output", help="Absolute .dxf output path")
    parser.add_argument("--job", help="Batch job JSON: one export per listed section")
    args = parser.parse_args(argv)
    if args.job:
        single = (args.input, args.plane, args.offset_mm, args.output)
        if any(value is not None for value in single):
            parser.error("--job cannot be combined with --input/--plane/--offset-mm/--output")
        return run_job(Path(args.job))
    if None in (args.input, args.plane, args.offset_mm, args.output):
        parser.error("--input, --plane, --offset-mm and --output are required without --job")

    try:
        import_service = ImportService(read_model, ImportSessions())
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
