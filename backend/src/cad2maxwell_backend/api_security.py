import hmac
from collections.abc import Awaitable, Callable

from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse
from starlette.responses import Response

from cad2maxwell_backend.config import Settings


def install_request_security(app: FastAPI, allowed_origins: tuple[str, ...]) -> None:
    settings = Settings()
    token = settings.session_token.get_secret_value() if settings.session_token else None

    @app.middleware("http")
    async def guard(
        request: Request, call_next: Callable[[Request], Awaitable[Response]],
    ) -> Response:
        origin = request.headers.get("Origin")
        if origin is not None and origin not in allowed_origins:
            return JSONResponse(status_code=403, content={"error": {
                "code": "origin_rejected",
                "message": "This origin cannot access the geometry service.",
            }})
        preflight = (request.method == "OPTIONS" and origin in allowed_origins
                     and "Access-Control-Request-Method" in request.headers)
        supplied_token = request.headers.get("X-Session-Token", "")
        if token and not preflight and not hmac.compare_digest(
            supplied_token.encode("utf-8"), token.encode("utf-8"),
        ):
            return JSONResponse(status_code=401, content={"error": {
                "code": "session_rejected",
                "message": "Restart CAD2Maxwell to reconnect to its service.",
            }})
        return await call_next(request)
