import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from pydantic import ValidationError

from cad2maxwell_backend.batch_job import load_job
from cad2maxwell_backend.export.archive import EXTRA_FILES, extra_texts
from cad2maxwell_backend.export.dxf import DxfExportError, export_dxf
from cad2maxwell_backend.geometry.model_reader import read_model
from cad2maxwell_backend.geometry.section import SectionError
from cad2maxwell_backend.models.exports import ExportOptions
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
        targets = [output_dir / f"{section.name}.dxf" for section in job.sections]
        extras = [[output_dir / f"{section.name}{Path(EXTRA_FILES[f]).suffix}"
                   for f in section.formats if f in EXTRA_FILES] for section in job.sections]
        planned = [*targets, *(t.with_suffix(".json") for t in targets),
                   *(path for paths in extras for path in paths)]
        clashes = [t.name for t in planned if t.exists()]
        if clashes:
            raise ValueError(f"outputs already exist: {', '.join(clashes)}")
        import_service = ImportService(read_model, ImportSessions())
        imported = import_service.import_path(job.input)
        section_service = SectionService(import_service.sessions, SectionSessions())
        for section, target, extra_paths in zip(job.sections, targets, extras, strict=True):
            created = section_service.create(SectionRequest(
                import_id=imported.import_id,
                plane=SectionPlane(kind=section.plane, offset_mm=section.offset_mm),
            ))
            options = ExportOptions(formats=section.formats)
            result = export_dxf(imported, created, target, options)
            texts = extra_texts(imported, created, options,
                                section_service.sections.get(created.section_id))
            for path in extra_paths:
                path.write_text(texts[next(n for n in texts if n.endswith(path.suffix))],
                                encoding="utf-8")
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
