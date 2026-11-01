import type { Vec3 } from "../section/plane";

export type StandardView = "iso" | "top" | "front" | "right";

/** Direction from the target to the camera; the model's +Z stays up on screen. */
export const VIEW_DIRECTIONS: Record<StandardView, Vec3> = {
  iso: [1, -1, 0.75],
  // A tiny -Y tilt keeps OrbitControls' Z-up frame defined when looking straight down.
  top: [0, -1e-6, 1],
  front: [0, -1, 0],
  right: [1, 0, 0],
};

/** Center and enclosing-sphere radius of an axis-aligned box. */
export function boundsSphere(min: Vec3, max: Vec3): { center: Vec3; radius: number } {
  return {
    center: [0, 1, 2].map((axis) => (min[axis] + max[axis]) / 2) as Vec3,
    radius: Math.hypot(...[0, 1, 2].map((axis) => max[axis] - min[axis])) / 2,
  };
}

/** Camera distance at which a sphere of `radius` fits the narrower field of view. */
export function fitDistance(radius: number, verticalFovDeg: number, aspect: number): number {
  const vertical = (verticalFovDeg * Math.PI) / 180;
  const horizontal = 2 * Math.atan(Math.tan(vertical / 2) * aspect);
  return (Math.max(radius, 1e-6) / Math.sin(Math.min(vertical, horizontal) / 2)) * 1.05;
}

/** Camera position looking at `center` from `direction` at the fitted distance. */
export function cameraPosition(
  center: Vec3, direction: Vec3, radius: number, verticalFovDeg: number, aspect: number,
): Vec3 {
  const length = Math.hypot(...direction) || 1;
  const distance = fitDistance(radius, verticalFovDeg, aspect);
  return [0, 1, 2].map((axis) => center[axis] + (direction[axis] / length) * distance) as Vec3;
}
