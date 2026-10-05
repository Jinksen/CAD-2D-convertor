import { useLayoutEffect, useRef, useState } from "react";

import { useWorkspaceStore } from "../../app/workspaceStore";
import type { ImportSummary } from "../step-import/contracts";
import type { SectionResponse } from "./contracts";
import { closedWirePath, curvePath } from "./rendering";
import { fitView, zoomView, type ViewBox } from "./viewCamera";

interface Props { section: SectionResponse; imported: ImportSummary; bounds: ViewBox }

export function SectionViewport({ section, imported, bounds }: Props) {
  const geometry = useRef<SVGGElement>(null);
  const [view, setView] = useState(() => fitView(bounds));
  const { selectedComponentId, selectComponent } = useWorkspaceStore();
  function fit() {
    const box = geometry.current && typeof geometry.current.getBBox === "function"
      ? geometry.current.getBBox() : null;
    setView(fitView(box && (box.width > 0 || box.height > 0)
      ? { x: box.x, y: -box.y - box.height, width: box.width, height: box.height } : bounds));
  }
  // Fit the rendered analytic paths; the source assembly can be much larger than its section.
  // This component is remounted for every computed section.
  useLayoutEffect(() => { fit(); }, []); // eslint-disable-line react-hooks/exhaustive-deps
  function color(id: string) {
    const source = imported.components.find((item) => item.id === id);
    return selectedComponentId === id ? "#ffffff" : source?.source_color
      ? `rgb(${source.source_color.map(String).join(",")})` : "#79c3f5";
  }
  return <>
    <div className="section-view-tools" aria-label="2D view controls">
      <button type="button" onClick={fit}>Fit section</button>
      <button type="button" aria-label="Zoom in" onClick={() => { setView(zoomView(view, 0.8)); }}>+</button>
      <button type="button" aria-label="Zoom out" onClick={() => { setView(zoomView(view, 1.25)); }}>−</button>
      <span>Click a region to select its source body</span>
    </div>
    <svg className="section-canvas" aria-label="2D section geometry"
      viewBox={`${String(view.x)} ${String(view.y)} ${String(view.width)} ${String(view.height)}`}
      onWheel={(event) => { setView(zoomView(view, event.deltaY < 0 ? 0.9 : 1.1)); }}>
      <g ref={geometry} transform="scale(1,-1)" fill="none" strokeWidth={1}>
        {section.diagnostics.every((item) => item.severity !== "error") &&
          section.components.map((component) => {
            if (!component.wires.every((wire) => wire.closed && wire.role !== null)) return null;
            const source = imported.components.find((item) => item.id === component.component_id);
            return <path key={`${component.component_id}-fill`}
              aria-label={`${source?.display_name ?? component.component_id} filled region`}
              d={component.wires.map(closedWirePath).join(" ")} fill={color(component.component_id)}
              fillOpacity={0.28} fillRule="evenodd" stroke="none"
              onClick={() => { selectComponent(component.component_id); }} />;
          })}
        {section.components.flatMap((component) => component.wires.flatMap((wire, wireIndex) =>
          wire.curves.map((curve, curveIndex) => <path
            key={`${component.component_id}-${String(wireIndex)}-${String(curveIndex)}`}
            d={curvePath(curve)} stroke={color(component.component_id)} vectorEffect="non-scaling-stroke"
            onClick={() => { selectComponent(component.component_id); }} />)))}
      </g>
    </svg>
  </>;
}
