import { useRef } from "react";

import { SectionMessages, SeverityLabel } from "../section/SectionSettings";
import type { SectionRun } from "../section/useSectionRun";
import type { ImportSummary } from "./contracts";
import { Progress } from "./Progress";
import { useGlobalShortcuts } from "./useGlobalShortcuts";

interface Props {
  imported: ImportSummary | null;
  run: SectionRun | null;
  importStartedAt: number | null;
  notice: string | null;
  onCancelImport: () => void;
}

export function MessagesPane({ imported, run, importStartedAt, notice, onCancelImport }: Props) {
  const pane = useRef<HTMLElement>(null);
  // Ctrl+L focuses the Messages pane.
  useGlobalShortcuts((key, ctrl) => {
    if (!ctrl || key !== "l") return false;
    pane.current?.focus();
    return true;
  });
  const running = importStartedAt !== null || run?.startedAt != null;
  return <section ref={pane} className="pane messages" aria-label="Messages" tabIndex={-1}>
    <div className="pane-title">Messages</div>
    <div className="messages-list">
      {importStartedAt !== null && <Progress label="Importing" since={importStartedAt} onCancel={onCancelImport} />}
      {run?.startedAt != null && <Progress key={run.startedAt} label="Computing section" since={run.startedAt}
        onCancel={run.cancel} />}
      {[notice, run?.notice].filter(Boolean).map((text, index) =>
        <p key={index} role="status" className="message">{text}</p>)}
      {imported && imported.diagnostics.length > 0 ? imported.diagnostics.map((item, index) =>
        <p key={`${item.code}-${String(index)}`} className={`message ${item.severity}`}>
          <SeverityLabel severity={item.severity} />{item.message}
        </p>) : !running && <p className="message">{imported ? "Import completed without messages." : "No messages."}</p>}
      {run && <SectionMessages run={run} />}
    </div>
  </section>;
}
