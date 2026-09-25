from pathlib import Path

from OCP.BRep import BRep_Builder
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox
from OCP.gp import gp_Trsf, gp_Vec
from OCP.IFSelect import IFSelect_ReturnStatus
from OCP.Interface import Interface_Static
from OCP.Quantity import Quantity_Color, Quantity_TypeOfColor
from OCP.STEPCAFControl import STEPCAFControl_Writer
from OCP.TCollection import TCollection_ExtendedString
from OCP.TDataStd import TDataStd_Name
from OCP.TDocStd import TDocStd_Document
from OCP.TopLoc import TopLoc_Location
from OCP.TopoDS import TopoDS_Compound
from OCP.XCAFApp import XCAFApp_Application
from OCP.XCAFDoc import XCAFDoc_ColorType, XCAFDoc_DocumentTool


def _document(unit_in_metres: float = 0.001) -> TDocStd_Document:
    document = TDocStd_Document(TCollection_ExtendedString("XmlXCAF"))
    XCAFApp_Application.GetApplication_s().InitDocument(document)
    XCAFDoc_DocumentTool.SetLengthUnit_s(document, unit_in_metres)
    return document


def _name(label: object, value: str) -> None:
    TDataStd_Name.Set_s(label, TCollection_ExtendedString(value, True))  # type: ignore[arg-type]


def _write(document: TDocStd_Document, path: Path, step_unit: str | None = None) -> Path:
    XCAFDoc_DocumentTool.ShapeTool_s(document.Main()).UpdateAssemblies()
    writer = STEPCAFControl_Writer()
    writer.SetNameMode(True)
    writer.SetColorMode(True)
    previous_unit = Interface_Static.CVal_s("write.step.unit")
    try:
        if step_unit is not None:
            assert Interface_Static.SetCVal_s("write.step.unit", step_unit)
        assert writer.Transfer(document)
        assert writer.Write(str(path)) == IFSelect_ReturnStatus.IFSelect_RetDone
    finally:
        Interface_Static.SetCVal_s("write.step.unit", previous_unit)
    return path


def named_colored_box(path: Path) -> Path:
    document = _document()
    shapes = XCAFDoc_DocumentTool.ShapeTool_s(document.Main())
    colors = XCAFDoc_DocumentTool.ColorTool_s(document.Main())
    label = shapes.AddShape(BRepPrimAPI_MakeBox(10, 20, 30).Shape(), False)
    _name(label, "Cívka")
    colors.SetColor(
        label,
        Quantity_Color(0.8, 0.2, 0.1, Quantity_TypeOfColor.Quantity_TOC_sRGB),
        XCAFDoc_ColorType.XCAFDoc_ColorSurf,
    )
    return _write(document, path)


def repeated_occurrences(path: Path) -> Path:
    document = _document()
    shapes = XCAFDoc_DocumentTool.ShapeTool_s(document.Main())
    compound = TopoDS_Compound()
    BRep_Builder().MakeCompound(compound)
    assembly = shapes.AddShape(compound, True)
    _name(assembly, "Motor")
    product = shapes.AddShape(BRepPrimAPI_MakeBox(2, 3, 4).Shape(), False)
    _name(product, "Magnet")
    for index, x_offset in enumerate((0.0, 10.0), start=1):
        transform = gp_Trsf()
        transform.SetTranslation(gp_Vec(x_offset, 0, 0))
        occurrence = shapes.AddComponent(assembly, product, TopLoc_Location(transform))
        _name(occurrence, f"Magnet:{index}")
    return _write(document, path)


def overlapping_occurrences(path: Path) -> Path:
    document = _document()
    shapes = XCAFDoc_DocumentTool.ShapeTool_s(document.Main())
    compound = TopoDS_Compound()
    BRep_Builder().MakeCompound(compound)
    assembly = shapes.AddShape(compound, True)
    product = shapes.AddShape(BRepPrimAPI_MakeBox(10, 10, 20).Shape(), False)
    for index, x_offset in enumerate((0.0, 5.0), start=1):
        transform = gp_Trsf()
        transform.SetTranslation(gp_Vec(x_offset, 0, 0))
        occurrence = shapes.AddComponent(assembly, product, TopLoc_Location(transform))
        _name(occurrence, f"Block {index}")
    return _write(document, path)


def inch_box(path: Path) -> Path:
    document = _document(0.0254)
    shapes = XCAFDoc_DocumentTool.ShapeTool_s(document.Main())
    label = shapes.AddShape(BRepPrimAPI_MakeBox(1, 2, 3).Shape(), False)
    _name(label, "Inch Box")
    return _write(document, path, "INCH")


def unnamed_box(path: Path) -> Path:
    document = _document()
    shapes = XCAFDoc_DocumentTool.ShapeTool_s(document.Main())
    shapes.AddShape(BRepPrimAPI_MakeBox(3, 4, 5).Shape(), False)
    return _write(document, path)
