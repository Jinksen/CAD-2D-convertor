import { useEffect, useState } from "react";

import { BackendStatus } from "../features/backend-status/BackendStatus";
import { fetchBackendHealth } from "../features/backend-status/client";
import type { BackendHealth } from "../features/backend-status/contracts";
import { StepImportWorkspace } from "../features/step-import/StepImportWorkspace";
import "./App.css";
import { useWorkspaceStore } from "./workspaceStore";

export const HEALTH_RETRY_MS = 1000;

export function App() {
  const [health, setHealth] = useState<BackendHealth>({ state: "offline" });
  const imported = useWorkspaceStore((state) => state.imported);
  const sectionStats = useWorkspaceStore((state) => state.sectionStats);

  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    // The desktop app starts the geometry service with the window, so keep asking until it answers.
    const check = () => {
      void fetchBackendHealth().then((result) => {
        if (!active) return;
        setHealth(result);
        if (result.state !== "online") timer = setTimeout(check, HEALTH_RETRY_MS);
      });
    };
    check();
    return () => {
      active = false;
      clearTimeout(timer);
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
