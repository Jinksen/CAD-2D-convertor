export type Point2 = [number, number];

export type SectionCurve =
  | { type: "line"; start: Point2; end: Point2 }
  | { type: "circle"; center: Point2; radius: number; x_axis: Point2; y_axis: Point2; start_parameter: number; end_parameter: number }
  | { type: "ellipse"; center: Point2; major_radius: number; minor_radius: number; x_axis: Point2; y_axis: Point2; start_parameter: number; end_parameter: number };

export interface SectionResponse {
  section_id: string;
  import_id: string;
  plane: { kind: "XY" | "XZ" | "YZ"; offset_mm: number };
  components: { component_id: string; wires: { closed: boolean; role: "outer" | "hole" | null; curves: SectionCurve[] }[] }[];
  diagnostics: { code: string; severity: "warning" | "error"; message: string; component_id: string | null }[];
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function point(value: unknown): value is Point2 {
  return Array.isArray(value) && value.length === 2 && value.every(
    (n) => typeof n === "number" && Number.isFinite(n),
  );
}

function curve(value: unknown): value is SectionCurve {
  if (!record(value)) return false;
  if (value.type === "line") return point(value.start) && point(value.end);
  if (value.type !== "circle" && value.type !== "ellipse") return false;
  const radius = value.type === "circle" ? value.radius : value.major_radius;
  return point(value.center) && point(value.x_axis) && point(value.y_axis) &&
    typeof radius === "number" && Number.isFinite(radius) && radius > 0 &&
    (value.type === "circle" || (typeof value.minor_radius === "number" &&
      Number.isFinite(value.minor_radius) && value.minor_radius > 0)) &&
    typeof value.start_parameter === "number" && Number.isFinite(value.start_parameter) &&
    typeof value.end_parameter === "number" && Number.isFinite(value.end_parameter);
}

export function isSectionResponse(value: unknown): value is SectionResponse {
  return record(value) && typeof value.section_id === "string" &&
    typeof value.import_id === "string" && record(value.plane) &&
    (value.plane.kind === "XY" || value.plane.kind === "XZ" || value.plane.kind === "YZ") &&
    typeof value.plane.offset_mm === "number" && Number.isFinite(value.plane.offset_mm) &&
    Array.isArray(value.components) && value.components.every((component: unknown) =>
      record(component) && typeof component.component_id === "string" &&
      Array.isArray(component.wires) && component.wires.every((wire: unknown) =>
        record(wire) && typeof wire.closed === "boolean" &&
        (wire.role === null || wire.role === "outer" || wire.role === "hole") &&
        Array.isArray(wire.curves) && wire.curves.every(curve))) &&
    Array.isArray(value.diagnostics) && value.diagnostics.every((diagnostic: unknown) =>
      record(diagnostic) && typeof diagnostic.code === "string" &&
      (diagnostic.severity === "warning" || diagnostic.severity === "error") &&
      typeof diagnostic.message === "string" &&
      (diagnostic.component_id === null || typeof diagnostic.component_id === "string"));
}
