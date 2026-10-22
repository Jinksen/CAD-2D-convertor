import type { ReactNode } from "react";

import type { SectionPlaneKind } from "../features/section/plane";
import type { SectionRun } from "../features/section/useSectionRun";

const planes = [
  { kind: "XY", label: "XY" }, { kind: "XZ", label: "XZ" }, { kind: "YZ", label: "YZ" },
  { kind: "custom", label: "Custom" },
] as const;

function RibbonGroup({ label, children }: { label: string; children: ReactNode }) {
  return <div className="ribbon-group" role="group" aria-label={label}>
    <div className="ribbon-group-items">{children}</div>
    <span className="ribbon-group-label" aria-hidden="true">{label}</span>
  </div>;
}

interface RibbonProps {
  hasModel: boolean;
  activePlane: SectionPlaneKind;
  run: SectionRun | null;
  onOpen: () => void;
  onPlane: (plane: SectionPlaneKind) => void;
  onCompute: () => void;
  onExport: () => void;
}

export function Ribbon({ hasModel, activePlane, run, onOpen, onPlane, onCompute, onExport }: RibbonProps) {
  return <header className="ribbon" aria-label="Application toolbar">
    <RibbonGroup label="Model">
      <button type="button" className="ribbon-button" onClick={onOpen}>
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinejoin="round" aria-hidden="true"><path d="M4 7h6l2 2h8v10H4z" /></svg>
        Open STEP
      </button>
    </RibbonGroup>
    <RibbonGroup label="Section plane">
      {planes.map((plane) => (
        <button
          type="button"
          disabled={!hasModel}
          className="ribbon-toggle"
          aria-pressed={activePlane === plane.kind}
          title={plane.kind === "custom" ? "Tilted section plane, for example aligned to a sheet" : undefined}
          onClick={() => { onPlane(plane.kind); }}
          key={plane.kind}
        >
          {plane.label}
        </button>
      ))}
    </RibbonGroup>
    <RibbonGroup label="Run">
      <button type="button" className="ribbon-button" disabled={!run || run.busy} onClick={onCompute}
        title="Compute the exact section with the plane and offset from Settings">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinejoin="round" aria-hidden="true"><path d="M3 15 12 10l9 5-9 5z" /><path d="M12 3v14" strokeDasharray="2 2" /></svg>
        {run?.loading ? "Computing…" : "Compute"}
      </button>
    </RibbonGroup>
    <RibbonGroup label="Export">
      <button type="button" className="ribbon-button" disabled={!run?.exportable || run.exporting} onClick={onExport}
        title={run?.exportable ? "Save the draft DXF bundle (section.dxf + section.json)"
          : "Compute a section whose regions are all closed first"}>
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M12 4v10" /><path d="m8 10 4 4 4-4" /><path d="M5 19h14" /></svg>
        {run?.exporting ? "Exporting…" : "DXF"}
      </button>
    </RibbonGroup>
  </header>;
}
