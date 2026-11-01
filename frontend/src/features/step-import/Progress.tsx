import { useEffect, useState } from "react";

interface ProgressProps {
  label: string;
  /** Start of the operation, epoch milliseconds. */
  since: number;
  onCancel: () => void;
}

/** A running operation in the Messages pane: "Importing… 12 s" with a Cancel button. */
export function Progress({ label, since, onCancel }: ProgressProps) {
  const [now, setNow] = useState(since);
  useEffect(() => {
    const timer = setInterval(() => { setNow(Date.now()); }, 1000);
    return () => { clearInterval(timer); };
  }, []);
  return <p className="message">
    <span className="message-severity">running</span>{label}… {Math.max(0, Math.floor((now - since) / 1000))} s
    <button type="button" className="message-link" onClick={onCancel}>Cancel</button>
  </p>;
}
