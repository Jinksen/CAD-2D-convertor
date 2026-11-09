import sys
from pathlib import Path

from cad2maxwell_backend.geometry.step_importer import read_step
from cad2maxwell_backend.models.sections import SectionPlane, SectionRequest
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.section_service import SectionService
from cad2maxwell_backend.services.section_sessions import SectionSessions

REPOSITORY = Path(__file__).resolve().parents[2]
DEMO = REPOSITORY / "docs" / "demo" / "demo-motor.step"

sys.path.insert(0, str(REPOSITORY / "scripts" / "demo"))
from make_demo_motor import build  # noqa: E402


def test_demo_motor_imports_with_names_and_colours() -> None:
    model = read_step(DEMO)

    names = [body.source_name for body in model.bodies]
    assert len(names) == 15
    assert names[0] == "Stator" and "Rotor" in names and "Shaft" in names
    assert sum(name.startswith("Magnet_") for name in names) == 8
    assert sum(name.startswith("Coil_") for name in names) == 4
    assert all(body.source_color is not None for body in model.bodies)


def test_demo_motor_bodies_sit_in_two_named_sub_assemblies() -> None:
    paths = {body.source_name: body.hierarchy_path for body in read_step(DEMO).bodies}

    assert paths["Coil_1"] == ("Demo_Motor", "Stator_Assembly", "Coil_1")
    assert paths["Magnet_8"] == ("Demo_Motor", "Rotor_Assembly", "Magnet_8")


def test_demo_motor_mid_height_section_has_closed_regions_and_no_errors() -> None:
    model = read_step(DEMO)
    imports = ImportSessions()
    import_id = imports.put("demo", model.bodies)

    response = SectionService(imports, SectionSessions()).create(SectionRequest(
        import_id=import_id, plane=SectionPlane(kind="XY", offset_mm=25),
    ))

    assert not [item for item in response.diagnostics if item.severity == "error"]
    wires = [wire for component in response.components for wire in component.wires]
    assert all(wire.closed for wire in wires)
    # Each body is one region; the stator and the rotor each add one hole wire.
    assert sum(wire.role == "outer" for wire in wires) == 15
    assert sum(wire.role == "hole" for wire in wires) == 2


def test_demo_motor_generator_reproduces_the_committed_model(tmp_path: Path) -> None:
    # STEP style-record order is not byte-stable (OCCT); compare what the importer sees.
    def summary(path: Path) -> list[tuple[object, ...]]:
        bodies = read_step(path).bodies
        return [(body.source_name, body.source_color, body.bounds_mm) for body in bodies]

    assert summary(build(tmp_path / "demo-motor.step")) == summary(DEMO)
