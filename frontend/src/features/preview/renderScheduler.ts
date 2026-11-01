export interface FrameApi {
  request: (callback: () => void) => number;
  cancel: (handle: number) => void;
}

export interface RenderScheduler {
  /** Asks for one frame; repeated calls before it runs coalesce. */
  request: () => void;
  dispose: () => void;
}

/** Renders on demand only; skips frames while the canvas is hidden (zero size). */
export function createRenderScheduler(
  render: () => void, isVisible: () => boolean,
  frames: FrameApi = {
    request: (callback) => requestAnimationFrame(callback),
    cancel: (handle) => { cancelAnimationFrame(handle); },
  },
): RenderScheduler {
  let pending = 0;
  let disposed = false;
  return {
    request() {
      if (pending || disposed) return;
      pending = frames.request(() => {
        pending = 0;
        if (isVisible()) render();
      });
    },
    dispose() {
      disposed = true;
      if (pending) frames.cancel(pending);
      pending = 0;
    },
  };
}
