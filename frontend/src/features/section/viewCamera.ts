export interface ViewBox { x: number; y: number; width: number; height: number }

export function fitView(bounds: ViewBox): ViewBox {
  const margin = Math.max(bounds.width, bounds.height, 1) * 0.08;
  return { x: bounds.x - margin, y: bounds.y - margin,
    width: bounds.width + margin * 2, height: bounds.height + margin * 2 };
}

export function zoomView(view: ViewBox, factor: number,
  anchor = { x: view.x + view.width / 2, y: view.y + view.height / 2 }): ViewBox {
  return { x: anchor.x + (view.x - anchor.x) * factor,
    y: anchor.y + (view.y - anchor.y) * factor,
    width: view.width * factor, height: view.height * factor };
}

export interface ScreenSize { width: number; height: number }

/** Maps a point in the SVG element's pixel box to viewBox units (preserveAspectRatio xMidYMid meet). */
export function screenToView(view: ViewBox, size: ScreenSize, point: { x: number; y: number }) {
  const scale = Math.min(size.width / view.width, size.height / view.height);
  return { x: view.x + (point.x - (size.width - view.width * scale) / 2) / scale,
    y: view.y + (point.y - (size.height - view.height * scale) / 2) / scale };
}

/** ViewBox units per screen pixel under xMidYMid meet. */
export function unitsPerPixel(view: ViewBox, size: ScreenSize): number {
  return Math.max(view.width / size.width, view.height / size.height);
}

/** Moves the camera by (dx, dy) viewBox units; the geometry stays in place. */
export function panView(view: ViewBox, dx: number, dy: number): ViewBox {
  return { ...view, x: view.x + dx, y: view.y + dy };
}
