import { sectionViewBounds } from "./plane";
import { SectionViewport } from "./SectionViewport";
import type { SectionRun } from "./useSectionRun";

/** Graphics-pane drawing of the current section; its controls live in SectionSettings. */
export function SectionView({ run }: { run: SectionRun }) {
  const { section, imported, basis } = run;
  if (!section) {
    return <div className="section-view">
      <div className="empty-panel section-placeholder">
        <strong>{run.loading ? "Computing exact section…" : "No section computed"}</strong>
        <span>Choose a plane and offset in Settings, then click Compute Section.</span>
      </div>
    </div>;
  }
  return <div className="section-view">
    <SectionViewport key={section.section_id} section={section} imported={imported}
      bounds={basis ? sectionViewBounds(imported.bounds_mm, basis) : { x: 0, y: 0, width: 1, height: 1 }} />
    {section.components.length === 0 && <p className="section-empty">The plane does not intersect a body.</p>}
  </div>;
}
