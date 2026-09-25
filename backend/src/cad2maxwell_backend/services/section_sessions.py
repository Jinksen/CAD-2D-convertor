import secrets
from threading import RLock

from cad2maxwell_backend.geometry.section import ExactSectionWire
from cad2maxwell_backend.models.sections import SectionResponse


class SectionSessions:
    """Process-local ownership of exact section wires for validation and export."""

    def __init__(self) -> None:
        self._lock = RLock()
        self._entries: dict[str, dict[str, tuple[ExactSectionWire, ...]]] = {}
        self._responses: dict[str, SectionResponse] = {}

    def put(self, components: dict[str, tuple[ExactSectionWire, ...]]) -> str:
        with self._lock:
            section_id = secrets.token_urlsafe(24)
            while section_id in self._entries:
                section_id = secrets.token_urlsafe(24)
            self._entries[section_id] = components
            return section_id

    def get(self, section_id: str) -> dict[str, tuple[ExactSectionWire, ...]] | None:
        with self._lock:
            return self._entries.get(section_id)

    def keep_response(self, response: SectionResponse) -> None:
        with self._lock:
            if response.section_id not in self._entries:
                raise KeyError(response.section_id)
            self._responses[response.section_id] = response

    def get_response(self, section_id: str) -> SectionResponse | None:
        with self._lock:
            return self._responses.get(section_id)
