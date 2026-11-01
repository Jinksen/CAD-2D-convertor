import { describe, expect, it } from "vitest";
import { fitView, panView, screenToView, unitsPerPixel, zoomView } from "./viewCamera";

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

  it("maps screen pixels to viewBox units with letterboxing", () => {
    const view = { x: 10, y: 20, width: 100, height: 50 };
    // 400x400 px: scale 4, content 400x200 px centred with 100 px bars above and below.
    const size = { width: 400, height: 400 };
    expect(screenToView(view, size, { x: 0, y: 100 })).toEqual({ x: 10, y: 20 });
    expect(screenToView(view, size, { x: 200, y: 200 })).toEqual({ x: 60, y: 45 });
    expect(unitsPerPixel(view, size)).toBe(0.25);
    // 400x100 px: scale 2, content 200x100 px centred with 100 px bars left and right.
    expect(screenToView(view, { width: 400, height: 100 }, { x: 200, y: 50 })).toEqual({ x: 60, y: 45 });
    expect(unitsPerPixel(view, { width: 400, height: 100 })).toBe(0.5);
  });

  it("keeps the point under the cursor fixed when zooming at a screen anchor", () => {
    const view = { x: 10, y: 20, width: 100, height: 50 };
    const size = { width: 400, height: 400 };
    const cursor = { x: 300, y: 150 };
    const anchor = screenToView(view, size, cursor);
    const after = screenToView(zoomView(view, 0.8, anchor), size, cursor);
    expect(after.x).toBeCloseTo(anchor.x);
    expect(after.y).toBeCloseTo(anchor.y);
  });

  it("pans by viewBox units without resizing the view", () => {
    expect(panView({ x: 0, y: 0, width: 100, height: 50 }, -5, 2.5)).toEqual({ x: -5, y: 2.5, width: 100, height: 50 });
  });
});
