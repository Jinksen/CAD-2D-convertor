from cad2maxwell_backend.naming import assign_export_names, stable_component_id


def test_export_names_transliterate_and_deduplicate() -> None:
    assert assign_export_names(["Cívka", "Cívka", "***", "Cívka"]) == [
        "Civka", "Civka_02", "Component", "Civka_03",
    ]


def test_export_names_reserve_explicit_suffixes() -> None:
    assert assign_export_names(["Coil", "Coil_02", "Coil"]) == [
        "Coil", "Coil_02", "Coil_03",
    ]


def test_component_id_depends_on_content_occurrence_and_ordinal() -> None:
    original = stable_component_id("a" * 64, "0:1:2", 0)
    assert original == stable_component_id("a" * 64, "0:1:2", 0)
    assert original != stable_component_id("b" * 64, "0:1:2", 0)
    assert original != stable_component_id("a" * 64, "0:1:3", 0)
    assert original != stable_component_id("a" * 64, "0:1:2", 1)
    assert original.startswith("cmp_")
