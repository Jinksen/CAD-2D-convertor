import io
from collections.abc import Mapping
from pathlib import Path
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

from cad2maxwell_backend.export.csv_table import write_csv
from cad2maxwell_backend.export.dxf import export_dxf
from cad2maxwell_backend.export.femm import write_femm
from cad2maxwell_backend.export.gmsh import write_gmsh
from cad2maxwell_backend.export.plan import plan_export
from cad2maxwell_backend.export.pyaedt_script import write_pyaedt
from cad2maxwell_backend.export.regions import build_regions
from cad2maxwell_backend.export.svg import write_svg
from cad2maxwell_backend.geometry.section import ExactSectionWire
from cad2maxwell_backend.models.exports import ExportOptions
from cad2maxwell_backend.models.imports import ImportResponse
from cad2maxwell_backend.models.sections import SectionResponse

# format -> archive file name; the DXF and manifest are always written.
EXTRA_FILES = {"csv": "regions.csv", "svg": "section.svg", "femm": "section.lua",
               "gmsh": "section.geo", "pyaedt": "section_pyaedt.py"}


def export_archive(
    imported: ImportResponse, section: SectionResponse, options: ExportOptions | None = None,
    exact: Mapping[str, tuple[ExactSectionWire, ...]] | None = None,
) -> bytes:
    """Package the validated analytic draft and manifest for a browser download.

    `exact` (the section session wires) is required for the region-based formats.
    """
    options = options or ExportOptions()
    extras = [name for name in EXTRA_FILES if name in options.formats]
    with TemporaryDirectory(prefix="cad2maxwell-export-") as directory:
        result = export_dxf(imported, section, Path(directory) / "section.dxf", options)
        texts: dict[str, str] = {}
        if extras:
            plan = plan_export(imported, section, options)
            regions = build_regions(plan, exact, section.plane)
            writers = {
                "csv": lambda: write_csv(regions), "svg": lambda: write_svg(regions, plan),
                "femm": lambda: write_femm(regions, plan),
                "gmsh": lambda: write_gmsh(regions, plan),
                "pyaedt": lambda: write_pyaedt(regions, plan),
            }
            texts = {EXTRA_FILES[name]: writers[name]() for name in extras}
        output = io.BytesIO()
        with ZipFile(output, "w", compression=ZIP_DEFLATED) as archive:
            archive.write(result.dxf_path, "section.dxf")
            archive.write(result.manifest_path, "section.json")
            for file_name, text in texts.items():
                archive.writestr(file_name, text.encode("utf-8"))
        return output.getvalue()
