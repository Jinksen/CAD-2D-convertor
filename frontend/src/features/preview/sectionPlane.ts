import { Euler, Matrix4, Vector3 } from "three";

import { boundsCenter, customPlaneBasis, dot, type SectionPlaneKind, type Vec3 } from "../section/plane";
import type { BoundsMm } from "../step-import/contracts";

export type { PrincipalPlane } from "../section/plane";

export interface PlaneGuideFrame {
  center: [number, number, number];
  width: number;
  height: number;
  rotation: [number, number, number];
}

export function planeGuideFrame(
  bounds: BoundsMm, plane: SectionPlaneKind, offsetMm: number, normal: Vec3 = [0, 0, 1],
): PlaneGuideFrame {
  const middle = boundsCenter(bounds);
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
  if (plane === "YZ") return {
    center: [offsetMm, middle[1], middle[2]], width: span[2], height: span[1],
    rotation: [0, Math.PI / 2, 0],
  };
  const basis = customPlaneBasis(normal) ?? { normal: [0, 0, 1], xDir: [1, 0, 0], yDir: [0, 1, 0] };
  // Project the model midpoint onto the plane; size the guide to the model diagonal.
  const distance = dot(basis.normal, middle) - offsetMm;
  const center = middle.map((value, index) => value - distance * basis.normal[index]) as Vec3;
  const size = Math.max(Math.hypot(...span), 1);
  const rotation = new Euler().setFromRotationMatrix(new Matrix4().makeBasis(
    new Vector3(...basis.xDir), new Vector3(...basis.yDir), new Vector3(...basis.normal)));
  return { center, width: size, height: size, rotation: [rotation.x, rotation.y, rotation.z] };
}
