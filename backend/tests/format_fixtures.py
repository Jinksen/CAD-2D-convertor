from pathlib import Path

from OCP.BRep import BRep_Builder
from OCP.BRepTools import BRepTools
from OCP.IFSelect import IFSelect_ReturnStatus
from OCP.IGESCAFControl import IGESCAFControl_Writer
from OCP.IGESControl import IGESControl_Controller
from OCP.Interface import Interface_Static
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TopoDS import TopoDS_Compound
from step_fixtures import _document

from cad2maxwell_backend.geometry.model_reader import read_model


def iges_from_step(step: Path, iges: Path) -> Path:
    """Re-write a STEP fixture as IGES through the XCAF writer (names, colours, assembly)."""
    document = _document()
    reader = STEPCAFControl_Reader()
    reader.SetNameMode(True)
    reader.SetColorMode(True)
    assert reader.ReadFile(str(step)) == IFSelect_ReturnStatus.IFSelect_RetDone
    assert reader.Transfer(document)
    # Default mode writes solids as loose faces; mode 1 writes manifold solid B-Reps (MSBO).
    IGESControl_Controller.Init_s()  # registers the write.iges.* parameters
    assert Interface_Static.SetIVal_s("write.iges.brep.mode", 1)
    writer = IGESCAFControl_Writer()
    assert writer.Transfer(document)
    assert writer.Write(str(iges))
    return iges


def brep_from_step(step: Path, brep: Path) -> Path:
    model = read_model(step)
    compound = TopoDS_Compound()
    builder = BRep_Builder()
    builder.MakeCompound(compound)
    for body in model.bodies:
        builder.Add(compound, body.shape)
    assert BRepTools.Write_s(compound, str(brep))
    return brep
