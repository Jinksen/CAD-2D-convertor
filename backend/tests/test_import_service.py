import hashlib
from pathlib import Path
from typing import Never

import pytest
from step_fixtures import repeated_occurrences

from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.services.import_service import ImportService, InvalidImportPath
from cad2maxwell_backend.services.import_sessions import ImportSessions


@pytest.mark.parametrize("raw_path", ["part.step", "C:part.step", "", "C:/part.txt"])
def test_invalid_path_never_reaches_importer(raw_path: str) -> None:
    def unexpected_import(_path: Path) -> Never:
        raise AssertionError("importer must not run")

    service = ImportService(unexpected_import, ImportSessions())
    with pytest.raises(InvalidImportPath):
        service.import_path(raw_path)


def test_repeated_imports_keep_stable_component_identity_and_exact_shapes(tmp_path: Path) -> None:
    path = repeated_occurrences(tmp_path / "assembly.STP")
    sessions = ImportSessions()
    service = ImportService(read_step, sessions)

    first = service.import_path(str(path))
    second = service.import_path(str(path))

    assert first.sha256 == hashlib.sha256(path.read_bytes()).hexdigest()
    assert first.component_count == 2
    assert first.import_id != second.import_id
    assert [component.id for component in first.components] == [
        component.id for component in second.components
    ]
    assert len({component.id for component in first.components}) == 2
    assert [component.export_name for component in first.components] == [
        "Magnet_1", "Magnet_2",
    ]
    stored = sessions.get(first.import_id)
    assert stored is not None
    assert stored[0].shape is not None


def test_content_change_changes_component_id(tmp_path: Path) -> None:
    first_path = repeated_occurrences(tmp_path / "first.step")
    second_path = repeated_occurrences(tmp_path / "second.step")
    with second_path.open("ab") as stream:
        stream.write(b"\n")
    service = ImportService(read_step, ImportSessions())

    first = service.import_path(str(first_path))
    second = service.import_path(str(second_path))

    assert first.sha256 != second.sha256
    assert first.components[0].id != second.components[0].id
