import logging

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.models.health import HealthResponse
from cad2maxwell_backend.models.imports import (
    ImportErrorPayload,
    ImportErrorResponse,
    ImportRequest,
    ImportResponse,
)
from cad2maxwell_backend.services.import_service import ImportService, ImportServiceError
from cad2maxwell_backend.services.import_sessions import ImportSessions

LOGGER = logging.getLogger(__name__)

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

    return app


app = create_app()
