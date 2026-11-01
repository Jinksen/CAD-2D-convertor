from pathlib import Path
from threading import Event, Thread

from fastapi.testclient import TestClient
from step_fixtures import named_colored_box

from cad2maxwell_backend.api import create_app
from cad2maxwell_backend.domain.import_result import ImportedModel
from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.services.import_service import ImportService
from cad2maxwell_backend.services.import_sessions import ImportSessions


def test_health_answers_while_upload_import_runs(tmp_path: Path) -> None:
    started, release = Event(), Event()

    def slow_reader(path: Path) -> ImportedModel:
        started.set()
        assert release.wait(10)
        return read_step(path)

    content = named_colored_box(tmp_path / "box.step").read_bytes()
    statuses: dict[str, int] = {}
    # A shared portal means one event loop: a blocking import would stall /health.
    with TestClient(create_app(ImportService(slow_reader, ImportSessions()))) as client:
        upload = Thread(target=lambda: statuses.__setitem__("import", client.post(
            "/api/v1/imports/step/file?filename=box.step", content=content,
        ).status_code))
        upload.start()
        assert started.wait(10)
        health = Thread(target=lambda: statuses.__setitem__(
            "health", client.get("/api/v1/health").status_code,
        ))
        health.start()
        health.join(5)
        answered_during_import = statuses.get("health")
        release.set()
        upload.join(10)
        health.join(5)

    assert answered_during_import == 200
    assert statuses["import"] == 200
