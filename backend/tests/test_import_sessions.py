from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox

from cad2maxwell_backend.domain.import_result import ImportedBody
from cad2maxwell_backend.models.imports import BoundingBox
from cad2maxwell_backend.services.import_sessions import ImportSessions


def test_session_keeps_exact_shape_and_can_be_removed() -> None:
    solid = BRepPrimAPI_MakeBox(1, 2, 3).Shape()
    body = ImportedBody(
        shape=solid,
        occurrence_key="0:1:2",
        solid_ordinal=0,
        source_id="0:1:2",
        source_name="Box",
        name_provenance="product",
        hierarchy_path=("Box",),
        source_color=None,
        color_provenance=None,
        bounds_mm=BoundingBox(min_xyz=(0, 0, 0), max_xyz=(1, 2, 3)),
    )
    sessions = ImportSessions()

    import_id = sessions.put("a" * 64, (body,))

    stored = sessions.get(import_id)
    assert stored is not None
    assert stored[0].shape is solid
    assert sessions.get("missing") is None
    assert sessions.remove(import_id)
    assert sessions.get(import_id) is None
    assert not sessions.remove(import_id)


def test_import_ids_are_process_local_and_unique() -> None:
    sessions = ImportSessions()
    first = sessions.put("a" * 64, ())
    second = sessions.put("a" * 64, ())

    assert first != second
    assert sessions.get(first) == ()
    assert sessions.get(second) == ()
