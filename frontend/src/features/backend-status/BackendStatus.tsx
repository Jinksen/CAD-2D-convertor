import type { BackendHealth } from "./contracts";
import type { SectionStats } from "../../app/workspaceStore";

interface BackendStatusProps {
  health: BackendHealth;
  bodyCount?: number;
  sourceUnit?: string | null;
  sectionStats?: SectionStats | null;
}

export function BackendStatus({ health, bodyCount = 0, sourceUnit = null, sectionStats = null }: BackendStatusProps) {
  const isOnline = health.state === "online";

  return (
    <footer className="status-bar" role="status" aria-live="polite">
      <span className={isOnline ? "status-dot online" : "status-dot offline"} aria-hidden="true" />
      <span>Backend {isOnline ? "Online" : "Offline"}</span>
      {isOnline && <span className="status-detail">API {health.apiVersion}</span>}
      <span className="status-spacer" />
      <span>Bodies: {bodyCount}</span>
      <span>Regions: {sectionStats?.regions ?? "—"}</span>
      {sectionStats && <span>Errors: {sectionStats.errors} · Warnings: {sectionStats.warnings}</span>}
      <span>Units: mm</span>
      {bodyCount > 0 && <span>Source: {sourceUnit ?? "unknown"}</span>}
    </footer>
  );
}
