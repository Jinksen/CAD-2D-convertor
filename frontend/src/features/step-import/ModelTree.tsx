import { useWorkspaceStore } from "../../app/workspaceStore";
import type { SectionPlaneKind } from "../section/plane";
import { rgb } from "./color";
import type { ImportComponent, ImportSummary } from "./contracts";
import { useGlobalShortcuts } from "./useGlobalShortcuts";

/** The node shown in the Settings pane; null shows the selected body, else the model. */
export type SettingsNode = "section" | "format" | null;

/** Bodies grouped by their parent assembly path; one group means no extra tree level. */
function groupByParent(components: ImportComponent[]): [string, ImportComponent[]][] {
  const groups = new Map<string, ImportComponent[]>();
  for (const component of components) {
    const parent = component.hierarchy_path.slice(0, -1).join(" / ");
    groups.set(parent, [...(groups.get(parent) ?? []), component]);
  }
  return [...groups.entries()];
}

interface Props {
  imported: ImportSummary | null;
  node: SettingsNode;
  selectedComponentId: string | null;
  activePlane: SectionPlaneKind;
  onNode: (node: SettingsNode) => void;
  onSelectBody: (id: string | null) => void;
}

export function ModelTree({ imported, node, selectedComponentId, activePlane, onNode, onSelectBody }: Props) {
  const hiddenIds = useWorkspaceStore((state) => state.hiddenIds);
  const toggleHidden = useWorkspaceStore((state) => state.toggleHidden);
  const toggleIsolate = useWorkspaceStore((state) => state.toggleIsolate);
  // V hides or shows the selected body, I isolates it (again: show all).
  useGlobalShortcuts((key, ctrl) => {
    if (ctrl || !selectedComponentId || (key !== "v" && key !== "i")) return false;
    (key === "v" ? toggleHidden : toggleIsolate)(selectedComponentId);
    return true;
  });
  const displayNames = imported?.components.map((item) => item.display_name) ?? [];
  const duplicateNames = new Set(displayNames.filter((name, index) => displayNames.indexOf(name) !== index));

  function bodyButton(item: ImportComponent) {
    const duplicate = duplicateNames.has(item.display_name);
    const hidden = hiddenIds.includes(item.id);
    return <li key={item.id} className="tree-body"><button type="button" className="tree-node" aria-pressed={!node && selectedComponentId === item.id}
      aria-label={duplicate ? `${item.display_name} ${item.export_name}` : undefined}
      onClick={() => { onSelectBody(item.id); }}>
      <span className="color-swatch" style={{ backgroundColor: rgb(item.source_color) }} />
      {item.display_name}
      {duplicate && <span className="tree-secondary">{item.export_name}</span>}
    </button>
    <button type="button" className="tree-eye" aria-pressed={!hidden} title={hidden ? "Show body (V)" : "Hide body (V)"}
      aria-label={hidden ? "Show body" : "Hide body"}
      onClick={() => { toggleHidden(item.id); }}>{hidden ? "◌" : "◉"}</button></li>;
  }

  const groups = imported ? groupByParent(imported.components) : [];
  return <nav className="pane model-builder" aria-label="Model builder">
    <div className="pane-title">Model Builder</div>
    <div className="tree-global">
      <button type="button" className="tree-node" aria-pressed={node === "format"} onClick={() => { onNode("format"); }}>
        <svg className="tree-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          strokeWidth="1.6" strokeLinecap="round" aria-hidden="true"><path d="M3 17h18M6 17v-4M10 17v-2M14 17v-4M18 17v-2" /></svg>
        Units and Number Format
      </button>
    </div>
    {imported ? <div className="tree">
      <button type="button" className="tree-node" aria-pressed={!node && !selectedComponentId}
        onClick={() => { onSelectBody(null); }}>
        <svg className="tree-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          strokeWidth="1.6" aria-hidden="true"><path d="M6 3h9l4 4v14H6z" /></svg>
        {imported.path.split(/[\\/]/).pop()}
      </button>
      <details open>
        <summary className="tree-group">Bodies ({imported.component_count})</summary>
        {groups.length === 1 ? <ul className="tree-list">{groups[0][1].map(bodyButton)}</ul> :
          groups.map(([parent, bodies]) => <details open key={parent} className="tree-subgroup">
            <summary className="tree-group">{parent.split(" / ").pop() || "Root"} ({bodies.length})</summary>
            <ul className="tree-list">{bodies.map(bodyButton)}</ul>
          </details>)}
      </details>
      <button type="button" className="tree-node" aria-pressed={node === "section"}
        onClick={() => { onNode("section"); }}>
        <svg className="tree-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          strokeWidth="1.6" strokeLinejoin="round" aria-hidden="true"><path d="M3 15 12 10l9 5-9 5z" /></svg>
        Section {activePlane === "custom" ? "(custom plane)" : activePlane}
      </button>
    </div> : <div className="empty-panel"><strong>No model loaded</strong><span>Open a STEP file to inspect its bodies.</span></div>}
  </nav>;
}
