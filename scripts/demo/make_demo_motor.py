"""Write a synthetic, license-free motor assembly as STEP (named, coloured bodies in two
sub-assemblies, mm).

Run with the backend environment:  uv run --project backend python scripts/demo/make_demo_motor.py [OUTPUT]
Output is deterministic: no random input, and the STEP header timestamp/path are normalised.
"""

import math
import re
import sys
import tempfile
from pathlib import Path

from OCP.BRep import BRep_Builder
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder
from OCP.gp import gp_Ax1, gp_Dir, gp_Pnt, gp_Trsf
from OCP.IFSelect import IFSelect_ReturnStatus
from OCP.Quantity import Quantity_Color, Quantity_TypeOfColor
from OCP.STEPCAFControl import STEPCAFControl_Writer
from OCP.TCollection import TCollection_ExtendedString
from OCP.TDataStd import TDataStd_Name
from OCP.TDF import TDF_Label
from OCP.TDocStd import TDocStd_Document
from OCP.TopLoc import TopLoc_Location
from OCP.TopoDS import TopoDS_Compound, TopoDS_Shape
from OCP.XCAFApp import XCAFApp_Application
from OCP.XCAFDoc import XCAFDoc_ColorType, XCAFDoc_DocumentTool, XCAFDoc_ShapeTool

DEFAULT_OUTPUT = Path(__file__).resolve().parents[2] / "docs" / "demo" / "demo-motor.step"
SLOTS = 12
STACK_MM = 50.0  # axial length, z = 0 .. 50 (mid-height section at z = 25)
COIL_SLOTS = (0, 3, 6, 9)
MAGNETS = 8


def _rotated(shape: TopoDS_Shape, degrees: float) -> TopoDS_Shape:
    transform = gp_Trsf()
    transform.SetRotation(gp_Ax1(gp_Pnt(0, 0, 0), gp_Dir(0, 0, 1)), math.radians(degrees))
    return BRepBuilderAPI_Transform(shape, transform, True).Shape()


def _box(x0: float, x1: float, half_y: float, z0: float, z1: float) -> TopoDS_Shape:
    return BRepPrimAPI_MakeBox(gp_Pnt(x0, -half_y, z0), gp_Pnt(x1, half_y, z1)).Shape()


def _stator() -> TopoDS_Shape:
    shape: TopoDS_Shape = BRepAlgoAPI_Cut(
        BRepPrimAPI_MakeCylinder(60, STACK_MM).Shape(), BRepPrimAPI_MakeCylinder(36, STACK_MM).Shape()).Shape()
    for slot in range(SLOTS):
        cutter = _rotated(_box(35, 48, 3, -1, STACK_MM + 1), slot * 360 / SLOTS)
        shape = BRepAlgoAPI_Cut(shape, cutter).Shape()
    return shape


def _rotor() -> TopoDS_Shape:
    return BRepAlgoAPI_Cut(
        BRepPrimAPI_MakeCylinder(30, STACK_MM).Shape(), BRepPrimAPI_MakeCylinder(10, STACK_MM).Shape()).Shape()


Body = tuple[str, TopoDS_Shape, tuple[float, float, float]]


def _stator_assembly() -> list[Body]:
    bodies: list[Body] = [("Stator", _stator(), (0.62, 0.64, 0.68))]
    for index, slot in enumerate(COIL_SLOTS):
        coil = _rotated(_box(38, 46, 2, -5, STACK_MM + 5), slot * 360 / SLOTS)
        bodies.append((f"Coil_{index + 1}", coil, (0.80, 0.50, 0.20)))
    return bodies


def _rotor_assembly() -> list[Body]:
    bodies: list[Body] = [("Rotor", _rotor(), (0.45, 0.47, 0.52))]
    for index in range(MAGNETS):
        colour = (0.80, 0.15, 0.12) if index % 2 == 0 else (0.15, 0.30, 0.75)  # alternating N / S
        bodies.append((f"Magnet_{index + 1}", _rotated(_box(30, 34, 6, 5, STACK_MM - 5), index * 360 / MAGNETS), colour))
    bodies.append(("Shaft", BRepPrimAPI_MakeCylinder(10, STACK_MM + 30).Shape(), (0.85, 0.85, 0.87)))
    return bodies


def _assemblies() -> list[tuple[str, list[Body]]]:
    return [("Stator_Assembly", _stator_assembly()), ("Rotor_Assembly", _rotor_assembly())]


def _named(label: object, name: str) -> None:
    TDataStd_Name.Set_s(label, TCollection_ExtendedString(name))  # type: ignore[arg-type]


def _new_assembly(shapes: XCAFDoc_ShapeTool, name: str) -> TDF_Label:
    compound = TopoDS_Compound()
    BRep_Builder().MakeCompound(compound)
    label = shapes.AddShape(compound, True)
    _named(label, name)
    return label


def build(path: Path) -> Path:
    document = TDocStd_Document(TCollection_ExtendedString("XmlXCAF"))
    XCAFApp_Application.GetApplication_s().InitDocument(document)
    XCAFDoc_DocumentTool.SetLengthUnit_s(document, 0.001)
    shapes = XCAFDoc_DocumentTool.ShapeTool_s(document.Main())
    colours = XCAFDoc_DocumentTool.ColorTool_s(document.Main())
    motor = _new_assembly(shapes, "Demo_Motor")
    for assembly_name, bodies in _assemblies():
        assembly = _new_assembly(shapes, assembly_name)
        for name, shape, (red, green, blue) in bodies:
            product = shapes.AddShape(shape, False)
            _named(product, name)
            colours.SetColor(product, Quantity_Color(red, green, blue, Quantity_TypeOfColor.Quantity_TOC_sRGB),
                             XCAFDoc_ColorType.XCAFDoc_ColorSurf)
            _named(shapes.AddComponent(assembly, product, TopLoc_Location()), name)
        _named(shapes.AddComponent(motor, assembly, TopLoc_Location()), assembly_name)
    shapes.UpdateAssemblies()
    writer = STEPCAFControl_Writer()
    writer.SetNameMode(True)
    writer.SetColorMode(True)
    assert writer.Transfer(document)
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as scratch:
        raw = Path(scratch) / path.name
        assert writer.Write(str(raw)) == IFSelect_ReturnStatus.IFSelect_RetDone
        text = raw.read_text(encoding="utf-8")
    # The writer stamps the current time and the output path; pin both for reproducible bytes.
    text = re.sub(r"FILE_NAME\('[^']*','[^']*'", f"FILE_NAME('{path.name}','2000-01-01T00:00:00'", text, count=1)
    path.write_text(text, encoding="utf-8", newline="\n")
    return path


if __name__ == "__main__":
    print(build(Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUTPUT))
