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
