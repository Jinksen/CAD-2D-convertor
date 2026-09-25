export interface BoundsMm {
  min_xyz: [number, number, number];
  max_xyz: [number, number, number];
}

export interface ImportComponent {
  id: string;
  source_id: string | null;
  source_name: string | null;
  display_name: string;
  export_name: string;
  hierarchy_path: string[];
  source_color: [number, number, number] | null;
  name_provenance: "occurrence" | "product" | null;
  color_provenance: "instance" | "shape" | null;
  body_ordinal: number;
  bounds_mm: BoundsMm;
}

export interface ImportDiagnostic {
  code: string;
  severity: "info" | "warning" | "error";
  message: string;
  component_id: string | null;
}

export interface ImportSummary {
  import_id: string;
  path: string;
  sha256: string;
  source_unit: string | null;
  to_mm_scale: number | null;
  bounds_mm: BoundsMm;
  component_count: number;
  components: ImportComponent[];
  diagnostics: ImportDiagnostic[];
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function stringOrNull(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function coordinates(value: unknown): value is [number, number, number] {
  return Array.isArray(value) && value.length === 3 &&
    value.every((item) => typeof item === "number" && Number.isFinite(item));
}

function bounds(value: unknown): value is BoundsMm {
  if (!record(value)) return false;
  const { min_xyz, max_xyz } = value;
  return coordinates(min_xyz) && coordinates(max_xyz) &&
    min_xyz.every((low, index) => low <= max_xyz[index]);
}

function component(value: unknown): value is ImportComponent {
  return record(value) && typeof value.id === "string" &&
    stringOrNull(value.source_id) && stringOrNull(value.source_name) &&
    typeof value.display_name === "string" && typeof value.export_name === "string" &&
    Array.isArray(value.hierarchy_path) && value.hierarchy_path.every((part) => typeof part === "string") &&
    (value.source_color === null || (coordinates(value.source_color) && value.source_color.every((n) => Number.isInteger(n) && n >= 0 && n <= 255))) &&
    (value.name_provenance === null || value.name_provenance === "occurrence" || value.name_provenance === "product") &&
    (value.color_provenance === null || value.color_provenance === "instance" || value.color_provenance === "shape") &&
    typeof value.body_ordinal === "number" && Number.isInteger(value.body_ordinal) && value.body_ordinal >= 0 &&
    bounds(value.bounds_mm);
}

function diagnostic(value: unknown): value is ImportDiagnostic {
  return record(value) && typeof value.code === "string" &&
    (value.severity === "info" || value.severity === "warning" || value.severity === "error") &&
    typeof value.message === "string" && stringOrNull(value.component_id);
}

export function isImportSummary(value: unknown): value is ImportSummary {
  return record(value) && typeof value.import_id === "string" &&
    typeof value.path === "string" && typeof value.sha256 === "string" &&
    stringOrNull(value.source_unit) &&
    (value.to_mm_scale === null || (typeof value.to_mm_scale === "number" && Number.isFinite(value.to_mm_scale))) &&
    bounds(value.bounds_mm) && typeof value.component_count === "number" &&
    Number.isInteger(value.component_count) && Array.isArray(value.components) &&
    value.components.length === value.component_count && value.components.every(component) &&
    Array.isArray(value.diagnostics) && value.diagnostics.every(diagnostic);
}
