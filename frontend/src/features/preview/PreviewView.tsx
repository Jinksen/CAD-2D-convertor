import { Component, useEffect, useState, type ReactNode } from "react";

import { useNumberFormat } from "../../app/formatStore";
import { useWorkspaceStore } from "../../app/workspaceStore";
import { formatDirection, formatLength } from "../../lib/formatting";
import type { ImportSummary } from "../step-import/contracts";
import { fetchPreview } from "./client";
import type { PreviewMeshResponse } from "./contracts";
import { defaultOffset, normalize, planeAxis } from "../section/plane";
import { MeshCanvas } from "./MeshCanvas";

class GraphicsErrorBoundary extends Component<{ children: ReactNode }, { message: string | null }> {
  state: { message: string | null } = { message: null };

  static getDerivedStateFromError(error: unknown) {
    return { message: error instanceof Error ? error.message : "The 3D preview could not be displayed." };
  }

  render() {
    if (this.state.message) return <div className="preview-message" role="alert">{this.state.message}</div>;
    return this.props.children;
  }
}

export function PreviewView({ imported }: { imported: ImportSummary }) {
  const [preview, setPreview] = useState<PreviewMeshResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { selectedComponentId, selectComponent, activePlane, sectionOffsetMm, customNormal } = useWorkspaceStore();
  const offsetMm = sectionOffsetMm ?? defaultOffset(imported.bounds_mm, activePlane, customNormal);
  const format = useNumberFormat();
  const planeLabel = activePlane === "custom"
    ? <>Custom plane, <var className="vector">n</var> = {formatDirection(normalize(customNormal) ?? customNormal, format)},
      {" "}<var>d</var> = {formatLength(offsetMm, format)}</>
    : <>{activePlane} plane, <var>{planeAxis(activePlane)}</var> = {formatLength(offsetMm, format)}</>;

  useEffect(() => {
    let cancelled = false;
    void fetchPreview(imported.import_id).then((result) => {
      if (!cancelled) setPreview(result);
    }).catch((cause: unknown) => {
      if (!cancelled) setError(cause instanceof Error ? cause.message : "The 3D preview could not be loaded.");
    });
    return () => { cancelled = true; };
  }, [imported.import_id]);

  if (error) return <div className="preview-message" role="alert">{error}</div>;
  if (!preview) return <div className="preview-message">Generating 3D preview…</div>;
  return <div className="preview-view">
    <p className="preview-plane-label">{planeLabel}</p>
    <p className="preview-help">Drag to orbit · wheel to zoom · click a body to select</p>
    <GraphicsErrorBoundary>
      <MeshCanvas preview={preview} bounds={imported.bounds_mm}
        selectedComponentId={selectedComponentId} onSelect={selectComponent}
        plane={activePlane} offsetMm={offsetMm} normal={customNormal} />
    </GraphicsErrorBoundary>
  </div>;
}
