import logging
from pathlib import Path
from tempfile import TemporaryDirectory

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, Response

from cad2maxwell_backend.export.archive import export_archive
from cad2maxwell_backend.export.dxf import DxfExportError
from cad2maxwell_backend.geometry.preview_mesh import PreviewMeshError, generate_preview
from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.models.exports import ExportRequest
from cad2maxwell_backend.models.health import HealthResponse
from cad2maxwell_backend.models.imports import (
    ImportErrorPayload,
    ImportErrorResponse,
    ImportRequest,
    ImportResponse,
)
from cad2maxwell_backend.models.preview import PreviewMeshResponse
from cad2maxwell_backend.models.sections import SectionRequest, SectionResponse
from cad2maxwell_backend.services.import_service import ImportService, ImportServiceError
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.section_service import SectionError, SectionService, UnknownImport
from cad2maxwell_backend.services.section_sessions import SectionSessions

LOGGER = logging.getLogger(__name__)
MAX_UPLOAD_BYTES = 512 * 1024 * 1024

ALLOWED_ORIGINS = (
    "http://127.0.0.1:1420",
    "http://localhost:1420",
    "http://tauri.localhost",
    "tauri://localhost",
)


def create_app(import_service: ImportService | None = None) -> FastAPI:
    app = FastAPI(title="CAD2Maxwell Backend", version="0.1.0")
    service = import_service or ImportService(read_step, ImportSessions())
    app.state.import_service = service
    section_service = SectionService(service.sessions, SectionSessions())
    app.state.section_service = section_service
    app.add_middleware(
        CORSMiddleware,
        allow_origins=list(ALLOWED_ORIGINS),
        allow_credentials=False,
        allow_methods=["GET", "POST"],
        allow_headers=["Content-Type", "X-Session-Token"],
    )

    @app.get("/api/v1/health", response_model=HealthResponse)
    def health() -> HealthResponse:
        return HealthResponse()

    @app.post("/api/v1/imports/step", response_model=ImportResponse)
    def import_step(request: ImportRequest) -> ImportResponse | JSONResponse:
        try:
            return service.import_path(request.path)
        except ImportServiceError as exc:
            payload = ImportErrorResponse(
                error=ImportErrorPayload(code=exc.code, message=exc.public_message)
            )
            return JSONResponse(status_code=exc.status_code, content=payload.model_dump())
        except Exception as exc:
            LOGGER.error("Unexpected STEP import failure (%s)", type(exc).__name__)
            payload = ImportErrorResponse(error=ImportErrorPayload(
                code="internal_import_error", message="The STEP import could not be completed.",
            ))
            return JSONResponse(status_code=500, content=payload.model_dump())

    @app.post("/api/v1/imports/step/file", response_model=ImportResponse)
    async def import_step_file(request: Request, filename: str) -> ImportResponse | JSONResponse:
        if (not filename or Path(filename).name != filename or "\\" in filename
                or any(ord(char) < 32 or char in '<>:"|?*' for char in filename)
                or Path(filename).suffix.casefold() not in {".step", ".stp"}):
            return _error(422, "invalid_import_path", "Choose a STEP or STP file.")
        try:
            with TemporaryDirectory(prefix="cad2maxwell-import-") as directory:
                path = Path(directory) / filename
                total = 0
                with path.open("xb") as stream:
                    async for chunk in request.stream():
                        total += len(chunk)
                        if total > MAX_UPLOAD_BYTES:
                            return _error(413, "import_too_large", "The STEP file exceeds 512 MiB.")
                        stream.write(chunk)
                if total == 0:
                    return _error(400, "rejected_step_file", "The STEP file is empty.")
                imported = service.import_path(str(path))
                response = imported.model_copy(update={"path": filename})
                service.sessions.keep_response(response)
                return response
        except ImportServiceError as exc:
            return _error(exc.status_code, exc.code, exc.public_message)
        except OSError as exc:
            LOGGER.error("STEP upload failed (%s)", type(exc).__name__)
            return _error(500, "internal_import_error", "The STEP upload could not be completed.")
        except Exception as exc:
            LOGGER.error("Unexpected STEP upload failure (%s)", type(exc).__name__)
            return _error(500, "internal_import_error", "The STEP upload could not be completed.")

    @app.get("/api/v1/imports/{import_id}/preview", response_model=PreviewMeshResponse)
    def preview_import(import_id: str) -> PreviewMeshResponse | JSONResponse:
        imported = service.sessions.get_response(import_id)
        bodies = service.sessions.get(import_id)
        if imported is None or bodies is None:
            return _error(404, "unknown_import", "The import session was not found. Import again.")
        try:
            return generate_preview(imported, bodies)
        except PreviewMeshError as exc:
            return _error(422, "preview_unavailable", str(exc))
        except Exception as exc:
            LOGGER.error("Unexpected preview failure (%s)", type(exc).__name__)
            return _error(500, "internal_preview_error", "The 3D preview could not be completed.")

    @app.post("/api/v1/section", response_model=SectionResponse)
    def create_section(request: SectionRequest) -> SectionResponse | JSONResponse:
        try:
            return section_service.create(request)
        except UnknownImport:
            payload = ImportErrorResponse(error=ImportErrorPayload(
                code="unknown_import",
                message="The import session was not found. Import the file again.",
            ))
            return JSONResponse(status_code=404, content=payload.model_dump())
        except SectionError:
            payload = ImportErrorResponse(error=ImportErrorPayload(
                code="section_failed", message="The exact section could not be completed.",
            ))
            return JSONResponse(status_code=422, content=payload.model_dump())
        except Exception as exc:
            LOGGER.error("Unexpected section failure (%s)", type(exc).__name__)
            payload = ImportErrorResponse(error=ImportErrorPayload(
                code="internal_section_error", message="The section could not be completed.",
            ))
            return JSONResponse(status_code=500, content=payload.model_dump())

    @app.post("/api/v1/export/dxf")
    def download_dxf(request: ExportRequest) -> Response:
        imported = service.sessions.get_response(request.import_id)
        section = section_service.sections.get_response(request.section_id)
        if imported is None or section is None:
            return _error(404, "unknown_export_session", "Import and compute the section again.")
        if section.import_id != request.import_id:
            return _error(409, "export_session_mismatch", "The section belongs to another import.")
        try:
            archive = export_archive(imported, section)
        except DxfExportError as exc:
            return _error(422, "export_not_ready", str(exc))
        return Response(
            content=archive, media_type="application/zip",
            headers={"Content-Disposition": 'attachment; filename="section.zip"'},
        )

    return app


app = create_app()


def _error(status: int, code: str, message: str) -> JSONResponse:
    payload = ImportErrorResponse(error=ImportErrorPayload(code=code, message=message))
    return JSONResponse(status_code=status, content=payload.model_dump())
