import { useEffect, useState } from "react";

import { BackendStatus } from "../features/backend-status/BackendStatus";
import { fetchBackendHealth } from "../features/backend-status/client";
import type { BackendHealth } from "../features/backend-status/contracts";
import { StepImportWorkspace } from "../features/step-import/StepImportWorkspace";
import "./App.css";
import { useWorkspaceStore } from "./workspaceStore";

const planes = ["XY", "XZ", "YZ"] as const;

export function App() {
  const [health, setHealth] = useState<BackendHealth>({ state: "offline" });
  const { activePlane, setActivePlane, imported } = useWorkspaceStore();

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
      <header className="toolbar" aria-label="Application toolbar">
        <div className="brand">CAD2Maxwell</div>
        <span className="toolbar-label">STEP IMPORT</span>
        <button type="button" disabled>
          Save
        </button>
        <div className="toolbar-separator" />
        {planes.map((plane) => (
          <button
            type="button"
            className={activePlane === plane ? "active" : ""}
            aria-pressed={activePlane === plane}
            onClick={() => {
              setActivePlane(plane);
            }}
            key={plane}
          >
            {plane}
          </button>
        ))}
        <div className="toolbar-separator" />
      </header>

      <StepImportWorkspace />

      <BackendStatus health={health} bodyCount={imported?.component_count ?? 0} sourceUnit={imported?.source_unit} />
    </div>
  );
}
