import { useEffect, useState } from "react";

import { BackendStatus } from "../features/backend-status/BackendStatus";
import { fetchBackendHealth } from "../features/backend-status/client";
import type { BackendHealth } from "../features/backend-status/contracts";
import { StepImportWorkspace } from "../features/step-import/StepImportWorkspace";
import "./App.css";
import { useWorkspaceStore } from "./workspaceStore";

export function App() {
  const [health, setHealth] = useState<BackendHealth>({ state: "offline" });
  const { imported, sectionStats } = useWorkspaceStore();

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

      <StepImportWorkspace />

      <BackendStatus health={health} bodyCount={imported?.component_count ?? 0} sourceUnit={imported?.source_unit} sectionStats={sectionStats} />
    </div>
  );
}
