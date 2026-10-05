import type { BoundsMm } from "../step-import/contracts";

export type PrincipalPlane = "XY" | "XZ" | "YZ";

export interface PlaneGuideFrame {
  center: [number, number, number];
  width: number;
  height: number;
  rotation: [number, number, number];
}

export function planeGuideFrame(bounds: BoundsMm, plane: PrincipalPlane, offsetMm: number): PlaneGuideFrame {
  const middle = bounds.min_xyz.map((low, index) =>
    (low + bounds.max_xyz[index]) / 2) as [number, number, number];
  const span = bounds.max_xyz.map((high, index) =>
    Math.max((high - bounds.min_xyz[index]) * 1.08, 1));
  if (plane === "XY") return {
    center: [middle[0], middle[1], offsetMm], width: span[0], height: span[1],
    rotation: [0, 0, 0],
  };
  if (plane === "XZ") return {
    center: [middle[0], offsetMm, middle[2]], width: span[0], height: span[2],
    rotation: [Math.PI / 2, 0, 0],
  };
  return {
    center: [offsetMm, middle[1], middle[2]], width: span[2], height: span[1],
    rotation: [0, Math.PI / 2, 0],
  };
}
