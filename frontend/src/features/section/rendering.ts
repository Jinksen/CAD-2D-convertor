import type { Point2, SectionCurve } from "./contracts";

function ellipsePoint(
  center: Point2, xAxis: Point2, yAxis: Point2,
  major: number, minor: number, parameter: number,
): Point2 {
  return [
    center[0] + major * xAxis[0] * Math.cos(parameter) + minor * yAxis[0] * Math.sin(parameter),
    center[1] + major * xAxis[1] * Math.cos(parameter) + minor * yAxis[1] * Math.sin(parameter),
  ];
}

export function curvePath(curve: SectionCurve): string {
  if (curve.type === "line") {
    return `M ${String(curve.start[0])} ${String(curve.start[1])} L ${String(curve.end[0])} ${String(curve.end[1])}`;
  }
  const major = curve.type === "circle" ? curve.radius : curve.major_radius;
  const minor = curve.type === "circle" ? curve.radius : curve.minor_radius;
  const rotation = Math.atan2(curve.x_axis[1], curve.x_axis[0]) * 180 / Math.PI;
  const delta = curve.end_parameter - curve.start_parameter;
  const segments = Math.max(1, Math.ceil(Math.abs(delta) / Math.PI));
  const start = ellipsePoint(curve.center, curve.x_axis, curve.y_axis,
    major, minor, curve.start_parameter);
  const determinant = curve.x_axis[0] * curve.y_axis[1] - curve.x_axis[1] * curve.y_axis[0];
  const sweep = delta * determinant > 0 ? 1 : 0;
  const pieces = [`M ${String(start[0])} ${String(start[1])}`];
  for (let index = 1; index <= segments; index += 1) {
    const end = ellipsePoint(curve.center, curve.x_axis, curve.y_axis,
      major, minor, curve.start_parameter + delta * index / segments);
    pieces.push(`A ${String(major)} ${String(minor)} ${String(rotation)} 0 ${String(sweep)} ${String(end[0])} ${String(end[1])}`);
  }
  return pieces.join(" ");
}
