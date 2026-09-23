import type { BackendHealth } from "./contracts";

interface BackendStatusProps {
  health: BackendHealth;
}

export function BackendStatus({ health }: BackendStatusProps) {
  const isOnline = health.state === "online";

  return (
    <footer className="status-bar" role="status" aria-live="polite">
      <span className={isOnline ? "status-dot online" : "status-dot offline"} aria-hidden="true" />
      <span>Backend {isOnline ? "Online" : "Offline"}</span>
      {isOnline && <span className="status-detail">API {health.apiVersion}</span>}
      <span className="status-spacer" />
      <span>Bodies: 0</span>
      <span>Regions: 0</span>
      <span>Units: mm</span>
    </footer>
  );
}
