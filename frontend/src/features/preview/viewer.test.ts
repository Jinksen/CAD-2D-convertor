import { describe, expect, it, vi } from "vitest";

import { cameraPosition, fitDistance, VIEW_DIRECTIONS } from "./cameraViews";
import { createRenderScheduler } from "./renderScheduler";

function fakeFrames() {
  const queue = new Map<number, () => void>();
  let next = 1;
  return {
    api: {
      request: (callback: () => void) => { queue.set(next, callback); return next++; },
      cancel: (handle: number) => { queue.delete(handle); },
    },
    pending: () => queue.size,
    flush: () => { const callbacks = [...queue.values()]; queue.clear(); callbacks.forEach((run) => { run(); }); },
  };
}

describe("render scheduler", () => {
  it("renders nothing until a frame is requested", () => {
    const frames = fakeFrames();
    const render = vi.fn();
    createRenderScheduler(render, () => true, frames.api);

    frames.flush();

    expect(render).not.toHaveBeenCalled();
    expect(frames.pending()).toBe(0);
  });

  it("coalesces requests into one frame and then goes idle", () => {
    const frames = fakeFrames();
    const render = vi.fn();
    const scheduler = createRenderScheduler(render, () => true, frames.api);

    scheduler.request();
    scheduler.request();
    expect(frames.pending()).toBe(1);
    frames.flush();

    expect(render).toHaveBeenCalledTimes(1);
    expect(frames.pending()).toBe(0);
  });

  it("skips rendering while hidden", () => {
    const frames = fakeFrames();
    const render = vi.fn();
    let visible = false;
    const scheduler = createRenderScheduler(render, () => visible, frames.api);

    scheduler.request();
    frames.flush();
    expect(render).not.toHaveBeenCalled();

    visible = true;
    scheduler.request();
    frames.flush();
    expect(render).toHaveBeenCalledTimes(1);
  });

  it("cancels the pending frame on dispose", () => {
    const frames = fakeFrames();
    const render = vi.fn();
    const scheduler = createRenderScheduler(render, () => true, frames.api);

    scheduler.request();
    scheduler.dispose();
    scheduler.request();
    frames.flush();

    expect(render).not.toHaveBeenCalled();
  });
});

describe("camera views", () => {
  it("fits a sphere into the field of view", () => {
    expect(fitDistance(1, 90, 1)).toBeCloseTo(Math.SQRT2 * 1.05);
    expect(fitDistance(1, 90, 0.5)).toBeGreaterThan(fitDistance(1, 90, 1));
    expect(fitDistance(1, 90, 2)).toBeCloseTo(fitDistance(1, 90, 1));
  });

  it("places standard views on the expected side of the model", () => {
    const center: [number, number, number] = [10, 20, 30];
    const top = cameraPosition(center, VIEW_DIRECTIONS.top, 5, 45, 1);
    const front = cameraPosition(center, VIEW_DIRECTIONS.front, 5, 45, 1);
    const right = cameraPosition(center, VIEW_DIRECTIONS.right, 5, 45, 1);
    const distance = fitDistance(5, 45, 1);

    expect(top[2] - center[2]).toBeCloseTo(distance);
    expect(top[0]).toBe(10);
    expect(front[1] - center[1]).toBeCloseTo(-distance);
    expect(right[0] - center[0]).toBeCloseTo(distance);
    const iso = cameraPosition(center, VIEW_DIRECTIONS.iso, 5, 45, 1);
    expect(Math.hypot(iso[0] - 10, iso[1] - 20, iso[2] - 30)).toBeCloseTo(distance);
  });
});
