import pytest
from fastapi.testclient import TestClient

from cad2maxwell_backend.api import create_app


def test_launched_service_requires_its_session_token(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("CAD2MAXWELL_SESSION_TOKEN", "local-session-secret")
    client = TestClient(create_app())
    for token in (None, "wrong-session"):
        headers = {} if token is None else {"X-Session-Token": token}
        assert client.get("/api/v1/health", headers=headers).status_code == 401
        assert client.post("/api/v1/imports/step", json={}, headers=headers).status_code == 401
    assert client.get("/api/v1/health", headers={
        "X-Session-Token": "local-session-secret",
    }).status_code == 200


def test_external_origin_is_rejected_before_geometry_runs() -> None:
    client = TestClient(create_app())
    response = client.post("/api/v1/imports/step", json={}, headers={
        "Origin": "https://untrusted.example",
    })
    assert response.status_code == 403


def test_allowed_origin_can_preflight_an_authenticated_request(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("CAD2MAXWELL_SESSION_TOKEN", "local-session-secret")
    response = TestClient(create_app()).options("/api/v1/section", headers={
        "Origin": "http://tauri.localhost",
        "Access-Control-Request-Method": "POST",
        "Access-Control-Request-Headers": "content-type,x-session-token",
    })
    assert response.status_code == 200
    assert response.headers["access-control-allow-origin"] == "http://tauri.localhost"
