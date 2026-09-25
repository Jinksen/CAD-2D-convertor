import io
from pathlib import Path
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

from cad2maxwell_backend.export.dxf import export_dxf
from cad2maxwell_backend.models.imports import ImportResponse
from cad2maxwell_backend.models.sections import SectionResponse


def export_archive(imported: ImportResponse, section: SectionResponse) -> bytes:
    """Package the validated analytic draft and manifest for a browser download."""
    with TemporaryDirectory(prefix="cad2maxwell-export-") as directory:
        result = export_dxf(imported, section, Path(directory) / "section.dxf")
        output = io.BytesIO()
        with ZipFile(output, "w", compression=ZIP_DEFLATED) as archive:
            archive.write(result.dxf_path, "section.dxf")
            archive.write(result.manifest_path, "section.json")
        return output.getvalue()
