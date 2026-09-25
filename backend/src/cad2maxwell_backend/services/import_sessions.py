import secrets
from threading import RLock

from cad2maxwell_backend.domain.import_result import ImportedBody


class ImportSessions:
    """Process-local ownership of exact imported shapes."""

    def __init__(self) -> None:
        self._lock = RLock()
        self._entries: dict[str, tuple[str, tuple[ImportedBody, ...]]] = {}

    def put(self, source_sha256: str, bodies: tuple[ImportedBody, ...]) -> str:
        with self._lock:
            import_id = secrets.token_urlsafe(24)
            while import_id in self._entries:
                import_id = secrets.token_urlsafe(24)
            self._entries[import_id] = (source_sha256, bodies)
            return import_id

    def get(self, import_id: str) -> tuple[ImportedBody, ...] | None:
        with self._lock:
            entry = self._entries.get(import_id)
            return entry[1] if entry is not None else None

    def get_record(self, import_id: str) -> tuple[str, tuple[ImportedBody, ...]] | None:
        with self._lock:
            return self._entries.get(import_id)

    def remove(self, import_id: str) -> bool:
        with self._lock:
            return self._entries.pop(import_id, None) is not None
