import secrets
from collections import OrderedDict
from collections.abc import Callable
from threading import RLock

from cad2maxwell_backend.domain.import_result import ImportedBody
from cad2maxwell_backend.models.imports import ImportResponse
from cad2maxwell_backend.services.limits import MAX_IMPORTS


class ImportSessions:
    """Process-local ownership of exact imported shapes (LRU, bounded)."""

    def __init__(self, max_imports: int = MAX_IMPORTS) -> None:
        self._lock = RLock()
        self._max = max_imports
        self._entries: OrderedDict[str, tuple[str, tuple[ImportedBody, ...]]] = OrderedDict()
        self._responses: dict[str, ImportResponse] = {}
        self._on_remove: list[Callable[[str], None]] = []

    def on_remove(self, listener: Callable[[str], None]) -> None:
        """Register a cascade callback, called with the import ID on eviction or removal."""
        self._on_remove.append(listener)

    def put(self, source_sha256: str, bodies: tuple[ImportedBody, ...]) -> str:
        with self._lock:
            import_id = secrets.token_urlsafe(24)
            while import_id in self._entries:
                import_id = secrets.token_urlsafe(24)
            self._entries[import_id] = (source_sha256, bodies)
            while len(self._entries) > self._max:
                self.remove(next(iter(self._entries)))
            return import_id

    def get(self, import_id: str) -> tuple[ImportedBody, ...] | None:
        entry = self.get_record(import_id)
        return entry[1] if entry is not None else None

    def get_record(self, import_id: str) -> tuple[str, tuple[ImportedBody, ...]] | None:
        with self._lock:
            if import_id in self._entries:
                self._entries.move_to_end(import_id)
            return self._entries.get(import_id)

    def keep_response(self, response: ImportResponse) -> None:
        with self._lock:
            if response.import_id not in self._entries:
                raise KeyError(response.import_id)
            self._responses[response.import_id] = response

    def get_response(self, import_id: str) -> ImportResponse | None:
        with self._lock:
            return self._responses.get(import_id)

    def remove(self, import_id: str) -> bool:
        with self._lock:
            self._responses.pop(import_id, None)
            removed = self._entries.pop(import_id, None) is not None
            for listener in self._on_remove:
                listener(import_id)
            return removed
