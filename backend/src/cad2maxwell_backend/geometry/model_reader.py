from pathlib import Path

from cad2maxwell_backend.domain.import_result import ImportedModel
from cad2maxwell_backend.geometry.brep_importer import read_brep
from cad2maxwell_backend.geometry.iges_importer import read_iges
from cad2maxwell_backend.geometry.step_importer import read_step

IMPORT_SUFFIXES = frozenset({".step", ".stp", ".igs", ".iges", ".brep"})


def read_model(path: Path) -> ImportedModel:
    """Dispatch on the file suffix; the service has already validated it."""
    suffix = path.suffix.casefold()
    if suffix in {".igs", ".iges"}:
        return read_iges(path)
    if suffix == ".brep":
        return read_brep(path)
    return read_step(path)
