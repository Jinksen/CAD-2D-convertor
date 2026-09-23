from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from cad2maxwell_backend.models.health import HealthResponse

ALLOWED_ORIGINS = (
    "http://127.0.0.1:1420",
    "http://localhost:1420",
    "tauri://localhost",
)


def create_app() -> FastAPI:
    app = FastAPI(title="CAD2Maxwell Backend", version="0.1.0")
    app.add_middleware(
        CORSMiddleware,
        allow_origins=list(ALLOWED_ORIGINS),
        allow_credentials=False,
        allow_methods=["GET"],
        allow_headers=["Content-Type", "X-Session-Token"],
    )

    @app.get("/api/v1/health", response_model=HealthResponse)
    def health() -> HealthResponse:
        return HealthResponse()

    return app


app = create_app()
