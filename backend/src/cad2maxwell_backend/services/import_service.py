import hashlib
import ntpath
from collections.abc import Callable
from pathlib import Path

from cad2maxwell_backend.domain.import_result import ImportedModel
from cad2maxwell_backend.geometry.step_importer import StepImportError
from cad2maxwell_backend.models.imports import ImportComponent, ImportResponse
from cad2maxwell_backend.naming import assign_export_names, stable_component_id
from cad2maxwell_backend.services.import_sessions import ImportSessions


class ImportServiceError(Exception):
    status_code = 500
    code = "internal_import_error"
    public_message = "The STEP import could not be completed."


class InvalidImportPath(ImportServiceError):
    status_code = 422
    code = "invalid_import_path"
    public_message = "Provide an absolute path to a local STEP or STP file."


class MissingImportFile(ImportServiceError):
    status_code = 404
    code = "missing_import_file"
    public_message = "The STEP file was not found."


class UnreadableImportFile(ImportServiceError):
    status_code = 403
    code = "unreadable_import_file"
    public_message = "The STEP file cannot be read."


class RejectedStepFile(ImportServiceError):
    status_code = 400
    code = "rejected_step_file"
    public_message = "The file contains no importable STEP solids."


def _validated_path(raw_path: str) -> Path:
    _, path_tail = ntpath.splitdrive(raw_path)
    if any(ord(char) < 32 or char in '<>:"|?*' for char in path_tail):
        raise InvalidImportPath
    try:
        path = Path(raw_path)
    except ValueError as exc:
        raise InvalidImportPath from exc
    if not path.is_absolute() or path.suffix.casefold() not in {".step", ".stp"}:
        raise InvalidImportPath
    try:
        resolved = path.resolve(strict=False)
        if not resolved.exists():
            raise MissingImportFile
        if not resolved.is_file():
            raise InvalidImportPath
    except PermissionError as exc:
        raise UnreadableImportFile from exc
    except ValueError as exc:
        raise InvalidImportPath from exc
    except OSError as exc:
        raise InvalidImportPath from exc
    return resolved


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(block)
    except FileNotFoundError as exc:
        raise MissingImportFile from exc
    except OSError as exc:
        raise UnreadableImportFile from exc
    return digest.hexdigest()


class ImportService:
    def __init__(
        self,
        importer: Callable[[Path], ImportedModel],
        sessions: ImportSessions,
    ) -> None:
        self._importer = importer
        self.sessions = sessions

    def import_path(self, raw_path: str) -> ImportResponse:
        path = _validated_path(raw_path)
        source_sha256 = _sha256_file(path)
        try:
            model = self._importer(path)
        except StepImportError as exc:
            raise RejectedStepFile from exc
        except FileNotFoundError as exc:
            raise MissingImportFile from exc
        except PermissionError as exc:
            raise UnreadableImportFile from exc

        display_names = [
            body.source_name or f"Body {index}"
            for index, body in enumerate(model.bodies, start=1)
        ]
        export_names = assign_export_names(display_names)
        components = [
            ImportComponent(
                id=stable_component_id(
                    source_sha256, body.occurrence_key, body.solid_ordinal,
                ),
                source_id=body.source_id,
                source_name=body.source_name,
                display_name=display_name,
                export_name=export_name,
                hierarchy_path=list(body.hierarchy_path),
                source_color=body.source_color,
                name_provenance=body.name_provenance,
                color_provenance=body.color_provenance,
                body_ordinal=body.solid_ordinal,
                bounds_mm=body.bounds_mm,
            )
            for body, display_name, export_name in zip(
                model.bodies, display_names, export_names, strict=True,
            )
        ]
        import_id = self.sessions.put(source_sha256, model.bodies)
        return ImportResponse(
            import_id=import_id,
            path=str(path),
            sha256=source_sha256,
            source_unit=model.source_unit,
            to_mm_scale=model.to_mm_scale,
            bounds_mm=model.bounds_mm,
            component_count=len(components),
            components=components,
            diagnostics=list(model.diagnostics),
        )
