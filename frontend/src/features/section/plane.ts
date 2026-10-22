import type { BoundsMm } from "../step-import/contracts";

export type PrincipalPlane = "XY" | "XZ" | "YZ";
export type SectionPlaneKind = PrincipalPlane | "custom";
export type Vec3 = [number, number, number];

export type SectionPlaneRequest =
  | { kind: PrincipalPlane; offset_mm: number }
  | { kind: "custom"; origin_xyz: Vec3; normal_xyz: Vec3; x_dir_xyz: Vec3 };

export interface PlaneBasis { normal: Vec3; xDir: Vec3; yDir: Vec3 }

const principalBases: Record<PrincipalPlane, PlaneBasis> = {
  // Matches the backend plane frames, so 2D coordinates agree with the DXF.
  XY: { normal: [0, 0, 1], xDir: [1, 0, 0], yDir: [0, 1, 0] },
  XZ: { normal: [0, -1, 0], xDir: [1, 0, 0], yDir: [0, 0, 1] },
  YZ: { normal: [1, 0, 0], xDir: [0, 1, 0], yDir: [0, 0, 1] },
};

export function dot(a: Vec3, b: Vec3): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

function cross(a: Vec3, b: Vec3): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

export function normalize(vector: Vec3): Vec3 | null {
  const length = Math.hypot(...vector);
  if (!Number.isFinite(length) || length < 1e-12) return null;
  return [vector[0] / length, vector[1] / length, vector[2] / length];
}

/** A deterministic in-plane frame: x follows global X (or Y for planes facing X). */
export function customPlaneBasis(normal: Vec3): PlaneBasis | null {
  const unit = normalize(normal);
  if (!unit) return null;
  const reference: Vec3 = Math.abs(unit[0]) > 0.9 ? [0, 1, 0] : [1, 0, 0];
  const along = dot(reference, unit);
  const xDir = normalize([reference[0] - along * unit[0], reference[1] - along * unit[1],
    reference[2] - along * unit[2]]);
  if (!xDir) return null;
  return { normal: unit, xDir, yDir: cross(unit, xDir) };
}

export function planeBasis(kind: SectionPlaneKind, normal: Vec3): PlaneBasis | null {
  return kind === "custom" ? customPlaneBasis(normal) : principalBases[kind];
}

export function boundsCenter(bounds: BoundsMm): Vec3 {
  return bounds.min_xyz.map((low, index) => (low + bounds.max_xyz[index]) / 2) as Vec3;
}

/** The plane through the model midpoint; for custom planes, measured along the unit normal. */
export function defaultOffset(bounds: BoundsMm, kind: SectionPlaneKind, normal: Vec3): number {
  const center = boundsCenter(bounds);
  if (kind === "XY") return center[2];
  if (kind === "XZ") return center[1];
  if (kind === "YZ") return center[0];
  const unit = normalize(normal);
  return unit ? dot(unit, center) : 0;
}

export function sectionPlaneRequest(kind: SectionPlaneKind, offsetMm: number, normal: Vec3): SectionPlaneRequest | null {
  if (kind !== "custom") return { kind, offset_mm: offsetMm };
  const basis = customPlaneBasis(normal);
  if (!basis) return null;
  return {
    kind, normal_xyz: basis.normal, x_dir_xyz: basis.xDir,
    origin_xyz: basis.normal.map((value) => value * offsetMm) as Vec3,
  };
}

/** The model bounds projected into section coordinates, in the SVG view box convention. */
export function sectionViewBounds(bounds: BoundsMm, basis: PlaneBasis) {
  const xs: number[] = [];
  const ys: number[] = [];
  for (const x of [bounds.min_xyz[0], bounds.max_xyz[0]]) {
    for (const y of [bounds.min_xyz[1], bounds.max_xyz[1]]) {
      for (const z of [bounds.min_xyz[2], bounds.max_xyz[2]]) {
        xs.push(dot([x, y, z], basis.xDir));
        ys.push(dot([x, y, z], basis.yDir));
      }
    }
  }
  const [xMin, xMax, yMin, yMax] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  return { x: xMin, y: -yMax, width: xMax - xMin, height: yMax - yMin };
}

export function formatNormal(normal: Vec3): string {
  return normal.map((value) => value.toFixed(3)).join(", ");
}
