import { describe, expect, it } from "vitest";

import { curvePath, outlineColor } from "./rendering";

describe("analytic section rendering", () => {
  it("draws a full ellipse as two SVG arcs", () => {
    const path = curvePath({ type: "ellipse", center: [0, 0], major_radius: 10,
      minor_radius: 5, x_axis: [1, 0], y_axis: [0, 1],
      start_parameter: 0, end_parameter: Math.PI * 2 });
    expect(path.match(/ A /g)).toHaveLength(2);
    expect(path).toContain("A 10 5");
    expect(path).not.toContain("NaN");
  });

  it("darkens source colors too light to read on the white canvas", () => {
    expect(outlineColor([255, 255, 255])).toBe("rgb(97,97,97)");
    expect(outlineColor([240, 240, 200])).not.toBe("rgb(240,240,200)");
    expect(outlineColor([201, 82, 75])).toBe("rgb(201,82,75)");
  });
});
