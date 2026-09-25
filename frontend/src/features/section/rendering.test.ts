import { describe, expect, it } from "vitest";

import { curvePath } from "./rendering";

describe("analytic section rendering", () => {
  it("draws a full ellipse as two SVG arcs", () => {
    const path = curvePath({ type: "ellipse", center: [0, 0], major_radius: 10,
      minor_radius: 5, x_axis: [1, 0], y_axis: [0, 1],
      start_parameter: 0, end_parameter: Math.PI * 2 });
    expect(path.match(/ A /g)).toHaveLength(2);
    expect(path).toContain("A 10 5");
    expect(path).not.toContain("NaN");
  });
});
