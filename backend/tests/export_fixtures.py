from functools import cache
from pathlib import Path

from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.models.imports import ImportResponse
from cad2maxwell_backend.models.sections import SectionPlane, SectionRequest, SectionResponse
from cad2maxwell_backend.services.import_service import ImportService
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.section_service import SectionService
from cad2maxwell_backend.services.section_sessions import SectionSessions

DEMO = Path(__file__).resolve().parents[2] / "docs" / "demo" / "demo-motor.step"


@cache
def _demo() -> tuple[ImportResponse, SectionResponse, SectionSessions]:
    imports = ImportService(read_step, ImportSessions())
    imported = imports.import_path(str(DEMO))
    sessions = SectionSessions()
    section = SectionService(imports.sessions, sessions).create(SectionRequest(
        import_id=imported.import_id, plane=SectionPlane(kind="XY", offset_mm=25),
    ))
    return imported, section, sessions


def demo_section() -> tuple[ImportResponse, SectionResponse]:
    """The demo motor sectioned at mid height: 15 bodies, arcs, two hole wires."""
    imported, section, _ = _demo()
    return imported, section


def demo_exact() -> dict[str, tuple]:  # type: ignore[type-arg]
    _, section, sessions = _demo()
    return sessions.get(section.section_id) or {}


def ids_named(imported: ImportResponse, prefix: str) -> list[str]:
    return [c.id for c in imported.components if c.source_name and c.source_name.startswith(prefix)]
