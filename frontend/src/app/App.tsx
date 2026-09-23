import { useEffect, useState } from "react";

import { BackendStatus } from "../features/backend-status/BackendStatus";
import { fetchBackendHealth } from "../features/backend-status/client";
import type { BackendHealth } from "../features/backend-status/contracts";
import "./App.css";
import { useWorkspaceStore } from "./workspaceStore";

const planes = ["XY", "XZ", "YZ"] as const;

export function App() {
  const [health, setHealth] = useState<BackendHealth>({ state: "offline" });
  const { activePlane, setActivePlane } = useWorkspaceStore();

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
        <button type="button">Open STEP</button>
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
        <button type="button" disabled>
          Compute Section
        </button>
        <button type="button" disabled>
          Export DXF
        </button>
      </header>

      <div className="workspace">
        <nav className="panel model-tree" aria-label="Model tree">
          <div className="panel-title">MODEL TREE</div>
          <div className="empty-panel">
            <span className="empty-icon" aria-hidden="true">◇</span>
            <strong>No model loaded</strong>
            <span>Open a STEP file to inspect components.</span>
          </div>
        </nav>

        <main className="viewport" aria-label="Viewport">
          <div className="viewport-tabs" role="tablist" aria-label="Viewport mode">
            <button type="button" role="tab" aria-selected="true">3D VIEW</button>
            <button type="button" role="tab" aria-selected="false" disabled>2D SECTION</button>
          </div>
          <div className="viewport-empty">
            <div className="axis-mark" aria-hidden="true"><i /><i /><i /></div>
            <h1>Engineering viewport</h1>
            <p>Exact CAD geometry will appear here after import.</p>
          </div>
        </main>

        <aside className="panel properties" aria-label="Properties">
          <div className="panel-title">PROPERTIES</div>
          <dl>
            <dt>Selection</dt><dd>None</dd>
            <dt>Section plane</dt><dd>{activePlane}</dd>
            <dt>Offset</dt><dd>0.000 mm</dd>
          </dl>
        </aside>
      </div>

      <BackendStatus health={health} />
    </div>
  );
}
