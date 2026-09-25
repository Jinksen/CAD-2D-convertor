from dataclasses import replace

from cad2maxwell_backend.geometry.section import SectionError, section_shape
from cad2maxwell_backend.geometry.section_classification import classify_wires
from cad2maxwell_backend.models.sections import (
    SectionComponent,
    SectionDiagnostic,
    SectionRequest,
    SectionResponse,
)
from cad2maxwell_backend.naming import stable_component_id
from cad2maxwell_backend.services.import_sessions import ImportSessions
from cad2maxwell_backend.services.section_sessions import SectionSessions


class UnknownImport(ValueError):
    """The process-local import session is no longer available."""


class SectionService:
    def __init__(self, imports: ImportSessions, sections: SectionSessions) -> None:
        self._imports = imports
        self.sections = sections

    def create(self, request: SectionRequest) -> SectionResponse:
        record = self._imports.get_record(request.import_id)
        if record is None:
            raise UnknownImport
        source_sha256, bodies = record
        components: list[SectionComponent] = []
        exact = {}
        diagnostics: list[SectionDiagnostic] = []
        for body in bodies:
            component_id = stable_component_id(
                source_sha256, body.occurrence_key, body.solid_ordinal,
            )
            wires = section_shape(body.shape, request.plane)
            if not wires:
                continue
            if all(wire.dto.closed for wire in wires):
                roles = classify_wires(wires, request.plane)
                wires = tuple(
                    replace(wire, dto=wire.dto.model_copy(update={"role": role}))
                    for wire, role in zip(wires, roles, strict=True)
                )
            components.append(SectionComponent(
                component_id=component_id, wires=[wire.dto for wire in wires],
            ))
            exact[component_id] = wires
            if any(not wire.dto.closed for wire in wires):
                diagnostics.append(SectionDiagnostic(
                    code="open_section_wire", severity="error",
                    message="The section has an open wire; no gap was closed automatically.",
                    component_id=component_id,
                ))
        if not components:
            diagnostics.append(SectionDiagnostic(
                code="empty_section", severity="warning",
                message="The plane does not intersect any imported solid.",
            ))
        section_id = self.sections.put(exact)
        return SectionResponse(
            section_id=section_id, import_id=request.import_id, plane=request.plane,
            components=components, diagnostics=diagnostics,
        )


__all__ = ["SectionError", "SectionService", "UnknownImport"]
