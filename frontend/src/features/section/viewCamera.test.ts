import { describe, expect, it } from "vitest";
import { fitView, zoomView } from "./viewCamera";

describe("2D view camera", () => {
  it("fits thin sections into a finite padded view", () => {
    const view = fitView({ x: 100, y: -40, width: 0, height: 20 });
    expect(view.x).toBeLessThan(100);
    expect(view.y).toBeLessThan(-40);
    expect(view.x + view.width).toBeGreaterThan(100);
    expect(view.y + view.height).toBeGreaterThan(-20);
    expect(view.width).toBeGreaterThan(0);
  });

  it("zooms around the cursor without changing the geometry coordinates", () => {
    const view = { x: 0, y: 0, width: 100, height: 50 };
    expect(zoomView(view, 0.5, { x: 25, y: 10 })).toEqual({ x: 12.5, y: 5, width: 50, height: 25 });
    expect(view).toEqual({ x: 0, y: 0, width: 100, height: 50 });
  });
});
