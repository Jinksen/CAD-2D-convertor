import secrets
from collections import OrderedDict
from threading import RLock

from cad2maxwell_backend.geometry.section import ExactSectionWire
from cad2maxwell_backend.models.sections import SectionResponse
from cad2maxwell_backend.services.limits import MAX_SECTIONS


class SectionSessions:
    """Process-local ownership of exact section wires for validation and export."""

    def __init__(self, max_sections: int = MAX_SECTIONS) -> None:
        self._lock = RLock()
        self._max = max_sections
        self._entries: OrderedDict[str, dict[str, tuple[ExactSectionWire, ...]]] = OrderedDict()
        self._responses: dict[str, SectionResponse] = {}
        self._owner: dict[str, str] = {}
        self._by_key: dict[tuple[str, tuple[object, ...]], str] = {}

    def put(
        self, components: dict[str, tuple[ExactSectionWire, ...]],
        import_id: str = "", cache_key: tuple[object, ...] | None = None,
    ) -> str:
        with self._lock:
            section_id = secrets.token_urlsafe(24)
            while section_id in self._entries:
                section_id = secrets.token_urlsafe(24)
            self._entries[section_id] = components
            self._owner[section_id] = import_id
            if cache_key is not None:
                self._by_key[(import_id, cache_key)] = section_id
            while len(self._entries) > self._max:
                self._remove(next(iter(self._entries)))
            return section_id

    def get(self, section_id: str) -> dict[str, tuple[ExactSectionWire, ...]] | None:
        with self._lock:
            if section_id in self._entries:
                self._entries.move_to_end(section_id)
            return self._entries.get(section_id)

    def find_cached(self, import_id: str, cache_key: tuple[object, ...]) -> SectionResponse | None:
        with self._lock:
            section_id = self._by_key.get((import_id, cache_key))
            if section_id is None or self.get(section_id) is None:
                return None
            return self._responses.get(section_id)

    def keep_response(self, response: SectionResponse) -> None:
        with self._lock:
            if response.section_id not in self._entries:
                raise KeyError(response.section_id)
            self._responses[response.section_id] = response

    def get_response(self, section_id: str) -> SectionResponse | None:
        with self._lock:
            if section_id in self._entries:
                self._entries.move_to_end(section_id)
            return self._responses.get(section_id)

    def remove_import(self, import_id: str) -> None:
        """Drop every section computed from one import."""
        with self._lock:
            for section_id in [sid for sid, owner in self._owner.items() if owner == import_id]:
                self._remove(section_id)

    def _remove(self, section_id: str) -> None:
        self._entries.pop(section_id, None)
        self._responses.pop(section_id, None)
        self._owner.pop(section_id, None)
        self._by_key = {key: sid for key, sid in self._by_key.items() if sid != section_id}
