import { useEffect, useRef, useState, type ReactNode } from "react";

import { BackendStatus } from "../features/backend-status/BackendStatus";
import { fetchBackendHealth } from "../features/backend-status/client";
import type { BackendHealth } from "../features/backend-status/contracts";
import { StepImportWorkspace } from "../features/step-import/StepImportWorkspace";
import "./App.css";
import { useWorkspaceStore } from "./workspaceStore";

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

export function App() {
  const [health, setHealth] = useState<BackendHealth>({ state: "offline" });
  const { activePlane, setActivePlane, imported, sectionStats } = useWorkspaceStore();
  const fileInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    let active = true;
    void fetchBackendHealth().then((result) => {
      if (active) setHealth(result);
    });
    return () => {
      active = false;
    };
  }, []);

  return (
    <div className="application-shell">
      <div className="title-bar">
        <strong>CAD2Maxwell</strong>
        <span className="title-bar-file">{imported ? imported.path.split(/[\\/]/).pop() : "No model"}</span>
      </div>
      <header className="ribbon" aria-label="Application toolbar">
        <RibbonGroup label="Model">
          <button type="button" className="ribbon-button" onClick={() => { fileInput.current?.click(); }}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
              strokeLinejoin="round" aria-hidden="true"><path d="M4 7h6l2 2h8v10H4z" /></svg>
            Open STEP
          </button>
        </RibbonGroup>
        <RibbonGroup label="Section plane">
          {planes.map((plane) => (
            <button
              type="button"
              disabled={!imported}
              className="ribbon-toggle"
              aria-pressed={activePlane === plane.kind}
              title={plane.kind === "custom" ? "Tilted section plane, for example aligned to a sheet" : undefined}
              onClick={() => {
                setActivePlane(plane.kind);
              }}
              key={plane.kind}
            >
              {plane.label}
            </button>
          ))}
        </RibbonGroup>
      </header>

      <StepImportWorkspace fileInputRef={fileInput} />

      <BackendStatus health={health} bodyCount={imported?.component_count ?? 0} sourceUnit={imported?.source_unit} sectionStats={sectionStats} />
    </div>
  );
}
