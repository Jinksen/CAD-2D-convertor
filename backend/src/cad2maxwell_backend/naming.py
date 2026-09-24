import hashlib
import re
import unicodedata

_LATIN_FALLBACKS = str.maketrans(
    {"Æ": "AE", "æ": "ae", "Ð": "D", "ð": "d", "Đ": "D", "đ": "d",
     "Ł": "L", "ł": "l", "Ø": "O", "ø": "o", "Œ": "OE", "œ": "oe",
     "Þ": "Th", "þ": "th", "ß": "ss"}
)


def _safe_name(name: str) -> str:
    latin = unicodedata.normalize("NFKD", name.translate(_LATIN_FALLBACKS))
    ascii_name = "".join(char for char in latin if not unicodedata.combining(char))
    safe = re.sub(r"[^A-Za-z0-9_-]+", "_", ascii_name).strip("_-")
    return safe or "Component"


def assign_export_names(display_names: list[str]) -> list[str]:
    bases = [_safe_name(name) for name in display_names]
    reserved = set(bases)
    used: set[str] = set()
    output: list[str] = []
    for base in bases:
        candidate = base
        if candidate in used:
            suffix = 2
            while f"{base}_{suffix:02d}" in used | reserved:
                suffix += 1
            candidate = f"{base}_{suffix:02d}"
        used.add(candidate)
        output.append(candidate)
    return output


def stable_component_id(source_sha256: str, occurrence_key: str, solid_ordinal: int) -> str:
    if solid_ordinal < 0:
        raise ValueError("solid ordinal must be nonnegative")
    digest = hashlib.sha256()
    for part in (source_sha256, occurrence_key, str(solid_ordinal)):
        encoded = part.encode("utf-8")
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
    return f"cmp_{digest.hexdigest()[:32]}"
