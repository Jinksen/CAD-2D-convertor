import { SectionMessages } from "../section/SectionSettings";
import type { SectionRun } from "../section/useSectionRun";
import type { ImportSummary } from "./contracts";
import { Progress } from "./Progress";

interface Props {
  imported: ImportSummary | null;
  run: SectionRun | null;
  importStartedAt: number | null;
  notice: string | null;
  onCancelImport: () => void;
}

export function MessagesPane({ imported, run, importStartedAt, notice, onCancelImport }: Props) {
  return <section className="pane messages" aria-label="Messages">
    <div className="pane-title">Messages</div>
    <div className="messages-list">
      {importStartedAt !== null && <Progress label="Importing" since={importStartedAt} onCancel={onCancelImport} />}
      {run?.startedAt != null && <Progress key={run.startedAt} label="Computing section" since={run.startedAt}
        onCancel={run.cancel} />}
      {[notice, run?.notice].filter(Boolean).map((text, index) =>
        <p key={index} role="status" className="message">{text}</p>)}
      {imported && imported.diagnostics.length > 0 ? imported.diagnostics.map((item, index) =>
        <p key={`${item.code}-${String(index)}`} className={`message ${item.severity}`}>
          <span className="message-severity">{item.severity}</span>{item.message}
        </p>) : <p className="message">{imported ? "Import completed without messages." : "No messages."}</p>}
      {run && <SectionMessages run={run} />}
    </div>
  </section>;
}
