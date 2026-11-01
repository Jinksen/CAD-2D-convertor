import { useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";

import { useWorkspaceStore } from "../../app/workspaceStore";
import type { ImportSummary } from "../step-import/contracts";
import type { SectionResponse } from "./contracts";
import { useGlobalShortcuts } from "../step-import/useGlobalShortcuts";
import { closedWirePath, componentBounds, curvePath, outlineColor } from "./rendering";
import { fitView, panView, screenToView, unitsPerPixel, zoomView, type ViewBox } from "./viewCamera";

interface Props { section: SectionResponse; imported: ImportSummary; bounds: ViewBox }

const PAN_KEYS: Record<string, [number, number]> = {
  ArrowLeft: [-0.1, 0], ArrowRight: [0.1, 0], ArrowUp: [0, -0.1], ArrowDown: [0, 0.1],
};

export function SectionViewport({ section, imported, bounds }: Props) {
  const canvas = useRef<SVGSVGElement>(null);
  const geometry = useRef<SVGGElement>(null);
  const drag = useRef<{ x: number; y: number; view: ViewBox } | null>(null);
  const [view, setView] = useState(() => fitView(bounds));
  const selectedComponentId = useWorkspaceStore((state) => state.selectedComponentId);
  const selectComponent = useWorkspaceStore((state) => state.selectComponent);
  const relatedComponentId = useWorkspaceStore((state) => state.relatedComponentId);
  const hiddenIds = useWorkspaceStore((state) => state.hiddenIds);
  const focusRequest = useWorkspaceStore((state) => state.focusRequest);
  const handledFocus = useRef(focusRequest.seq);
  const sources = useMemo(() => new Map(imported.components.map((item) => [item.id, item])), [imported]);
  /** Fits the body when given and found in this section; otherwise everything. */
  function fit(componentId: string | null = null) {
    const own = section.components.find((item) => item.component_id === componentId);
    const target = own ? componentBounds(own) : null;
    if (target) {
      setView(fitView({ x: target.x, y: -target.y - target.height, width: target.width, height: target.height }));
      return;
    }
    const box = geometry.current && typeof geometry.current.getBBox === "function"
      ? geometry.current.getBBox() : null;
    setView(fitView(box && (box.width > 0 || box.height > 0)
      ? { x: box.x, y: -box.y - box.height, width: box.width, height: box.height } : bounds));
  }
  // Fit the rendered analytic paths; the source assembly can be much larger than its section.
  // This component is remounted for every computed section.
  useLayoutEffect(() => { fit(); }, []); // eslint-disable-line react-hooks/exhaustive-deps, react-hooks/set-state-in-effect
  // A diagnostic asked for this body; requests older than this (re)mount are ignored.
  useEffect(() => {
    if (focusRequest.seq === handledFocus.current) return;
    handledFocus.current = focusRequest.seq;
    fit(focusRequest.componentId);
  }, [focusRequest]); // eslint-disable-line react-hooks/exhaustive-deps
  // F / Shift+F anywhere while this view is the visible one.
  useGlobalShortcuts((key, ctrl, event) => {
    const svg = canvas.current;
    if (ctrl || key !== "f" || !svg?.isConnected || svg.closest("[hidden]")) return false;
    fit(event.shiftKey ? null : selectedComponentId);
    return true;
  });
  // React's onWheel is passive, so it cannot stop the page from scrolling.
  useEffect(() => {
    const svg = canvas.current;
    if (!svg) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const rect = svg.getBoundingClientRect();
      const factor = event.deltaY < 0 ? 0.9 : 1.1;
      setView((current) => rect.width > 0 && rect.height > 0
        ? zoomView(current, factor, screenToView(current, rect,
          { x: event.clientX - rect.left, y: event.clientY - rect.top }))
        : zoomView(current, factor));
    };
    svg.addEventListener("wheel", onWheel, { passive: false });
    return () => { svg.removeEventListener("wheel", onWheel); };
  }, []);
  function startPan(event: PointerEvent<SVGSVGElement>) {
    // Middle button pans anywhere; left button only on the background so region clicks still select.
    if (event.button !== 1 && !(event.button === 0 && event.target === event.currentTarget)) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { x: event.clientX, y: event.clientY, view };
  }
  function movePan(event: PointerEvent<SVGSVGElement>) {
    const start = drag.current;
    const rect = event.currentTarget.getBoundingClientRect();
    if (!start || rect.width <= 0 || rect.height <= 0) return;
    const scale = unitsPerPixel(start.view, rect);
    setView(panView(start.view, (start.x - event.clientX) * scale, (start.y - event.clientY) * scale));
  }
  function onKeyDown(event: KeyboardEvent<SVGSVGElement>) {
    const pan = PAN_KEYS[event.key] as [number, number] | undefined;
    if (pan) setView(panView(view, pan[0] * view.width, pan[1] * view.height));
    else if (event.key === "f" || event.key === "F") fit(event.shiftKey ? null : selectedComponentId);
    else if (event.key === "+" || event.key === "=") setView(zoomView(view, 0.8));
    else if (event.key === "-" || event.key === "_") setView(zoomView(view, 1.25));
    else return;
    event.preventDefault();
  }
  function color(id: string, part: "fill" | "outline") {
    const source = sources.get(id);
    if (selectedComponentId === id) return "#1a5fb4";
    if (relatedComponentId === id) return "var(--warning, #b35c00)";
    if (!source?.source_color) return "#4a6f8f";
    return part === "outline" ? outlineColor(source.source_color) : `rgb(${source.source_color.map(String).join(",")})`;
  }
  return <>
    <div className="section-view-tools" aria-label="2D view controls">
      <button type="button" title="Fit selection (F), fit all (Shift+F)" onClick={() => { fit(selectedComponentId); }}>Fit section</button>
      <button type="button" aria-label="Zoom in" title="Zoom in (+)" onClick={() => { setView(zoomView(view, 0.8)); }}>+</button>
      <button type="button" aria-label="Zoom out" title="Zoom out (−)" onClick={() => { setView(zoomView(view, 1.25)); }}>−</button>
      <span>Wheel: zoom · drag: pan · arrows: pan · F: fit selection · Shift+F: fit all · click: select</span>
    </div>
    <svg ref={canvas} className="section-canvas" aria-label="2D section geometry" tabIndex={0}
      viewBox={`${String(view.x)} ${String(view.y)} ${String(view.width)} ${String(view.height)}`}
      onKeyDown={onKeyDown} onPointerDown={startPan} onPointerMove={movePan}
      onMouseDown={(event) => { if (event.button === 1) event.preventDefault(); /* no autoscroll */ }}
      onPointerUp={() => { drag.current = null; }} onPointerCancel={() => { drag.current = null; }}>
      <g ref={geometry} transform="scale(1,-1)" fill="none" strokeWidth={1}>
        {section.diagnostics.every((item) => item.severity !== "error") &&
          section.components.map((component) => {
            if (hiddenIds.includes(component.component_id)) return null;
            if (!component.wires.every((wire) => wire.closed && wire.role !== null)) return null;
            const source = sources.get(component.component_id);
            return <path key={`${component.component_id}-fill`}
              aria-label={`${source?.display_name ?? component.component_id} filled region`}
              d={component.wires.map(closedWirePath).join(" ")} fill={color(component.component_id, "fill")}
              fillOpacity={0.28} fillRule="evenodd" stroke="none"
              onClick={() => { selectComponent(component.component_id); }} />;
          })}
        {section.components.flatMap((component) => component.wires.flatMap((wire, wireIndex) =>
          wire.curves.map((curve, curveIndex) => <path
            key={`${component.component_id}-${String(wireIndex)}-${String(curveIndex)}`}
            d={curvePath(curve)} stroke={color(component.component_id, "outline")} vectorEffect="non-scaling-stroke"
            data-hidden={hiddenIds.includes(component.component_id) || undefined}
            strokeOpacity={hiddenIds.includes(component.component_id) ? 0.2 : 1}
            pointerEvents={hiddenIds.includes(component.component_id) ? "none" : undefined}
            onClick={() => { selectComponent(component.component_id); }} />)))}
      </g>
    </svg>
  </>;
}
