import { describe, expect, it } from "vitest";
import { Euler, Vector3 } from "three";

import type { BoundsMm } from "../step-import/contracts";
import { planeGuideFrame } from "./sectionPlane";

const bounds: BoundsMm = { min_xyz: [0, 10, 20], max_xyz: [100, 50, 80] };

describe("3D section plane guide", () => {
  it("places an XY guide at the requested Z coordinate", () => {
    const frame = planeGuideFrame(bounds, "XY", 33);
    expect(frame.center).toEqual([50, 30, 33]);
    expect(frame.rotation).toEqual([0, 0, 0]);
    expect(frame.width).toBeGreaterThan(100);
    expect(frame.height).toBeGreaterThan(40);
  });

  it("places XZ and YZ guides on their respective offset axes", () => {
    expect(planeGuideFrame(bounds, "XZ", 25).center).toEqual([50, 25, 50]);
    expect(planeGuideFrame(bounds, "XZ", 25).rotation).toEqual([Math.PI / 2, 0, 0]);
    expect(planeGuideFrame(bounds, "YZ", 70).center).toEqual([70, 30, 50]);
    expect(planeGuideFrame(bounds, "YZ", 70).rotation).toEqual([0, Math.PI / 2, 0]);
  });

  it("covers the Y and Z extents of an unequal YZ section", () => {
    const frame = planeGuideFrame(bounds, "YZ", 70);
    const edge = new Vector3(frame.width, frame.height, 0).applyEuler(new Euler(...frame.rotation));
    expect(Math.abs(edge.y)).toBeCloseTo(43.2);
    expect(Math.abs(edge.z)).toBeCloseTo(64.8);
  });
});
