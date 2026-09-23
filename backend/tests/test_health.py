from fastapi.testclient import TestClient

from cad2maxwell_backend.api import create_app
from cad2maxwell_backend.config import Settings


def test_health_returns_versioned_typed_payload() -> None:
    response = TestClient(create_app()).get("/api/v1/health")

    assert response.status_code == 200
    assert response.json() == {
        "status": "ok",
        "service": "cad2maxwell-backend",
        "api_version": "v1",
    }


def test_external_host_is_not_configured() -> None:
    assert Settings().host == "127.0.0.1"


def test_windows_tauri_origin_is_allowed() -> None:
    response = TestClient(create_app()).get(
        "/api/v1/health",
        headers={"Origin": "http://tauri.localhost"},
    )

    assert response.headers["access-control-allow-origin"] == "http://tauri.localhost"
