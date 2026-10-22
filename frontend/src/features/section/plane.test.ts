import { describe, expect, it } from "vitest";

import type { BoundsMm } from "../step-import/contracts";
import { customPlaneBasis, defaultOffset, dot, sectionPlaneRequest, sectionViewBounds, type Vec3 } from "./plane";

const bounds: BoundsMm = { min_xyz: [0, 10, 20], max_xyz: [100, 50, 80] };

describe("section plane geometry", () => {
  it("builds an orthonormal frame for a tilted normal", () => {
    const basis = customPlaneBasis([0.331, 0.013, 0.944]);
    expect(basis).not.toBeNull();
    if (!basis) return;
    for (const vector of [basis.normal, basis.xDir, basis.yDir]) expect(Math.hypot(...vector)).toBeCloseTo(1, 12);
    expect(dot(basis.normal, basis.xDir)).toBeCloseTo(0, 12);
    expect(dot(basis.normal, basis.yDir)).toBeCloseTo(0, 12);
    expect(basis.xDir[0]).toBeGreaterThan(0.9);
  });

  it("matches the backend frames for axis-aligned normals", () => {
    expect(customPlaneBasis([0, 0, 2])).toEqual({ normal: [0, 0, 1], xDir: [1, 0, 0], yDir: [0, 1, 0] });
    const yz = customPlaneBasis([1, 0, 0]);
    expect(yz?.xDir).toEqual([0, 1, 0]);
    expect(yz?.yDir).toEqual([0, 0, 1]);
  });

  it("rejects a zero normal instead of guessing a plane", () => {
    expect(customPlaneBasis([0, 0, 0])).toBeNull();
    expect(sectionPlaneRequest("custom", 5, [0, 0, 0])).toBeNull();
  });

  it("places the default custom plane through the model midpoint", () => {
    const normal: Vec3 = [0, 3, 4];
    expect(defaultOffset(bounds, "custom", normal)).toBeCloseTo((30 * 3 + 50 * 4) / 5);
    expect(defaultOffset(bounds, "XZ", normal)).toBe(30);
  });

  it("sends the origin on the unit normal at the requested offset", () => {
    const request = sectionPlaneRequest("custom", 10, [0, 0, 2]);
    expect(request).toEqual({ kind: "custom", origin_xyz: [0, 0, 10], normal_xyz: [0, 0, 1], x_dir_xyz: [1, 0, 0] });
    expect(sectionPlaneRequest("XY", 3, [1, 0, 0])).toEqual({ kind: "XY", offset_mm: 3 });
  });

  it("projects model bounds into the section view box", () => {
    const basis = customPlaneBasis([0, 0, 1]);
    if (!basis) throw new Error("basis");
    expect(sectionViewBounds(bounds, basis)).toEqual({ x: 0, y: -50, width: 100, height: 40 });
  });
});
