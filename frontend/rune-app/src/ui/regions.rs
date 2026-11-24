//! Exact region metrics of the selected body in the shown section (properties pane). The
//! numbers are OpenCascade's, fetched from the service and only formatted here.

use rune::tok::{Theme, space, type_};
use rune::{El, col, text};

use super::{label_value, severity_color};
use crate::api::dto::Severity;
use crate::state::AppState;
use crate::state::section::regions::Load;

/// The pane for `body`, or None when the shown section has no such body.
pub fn rows(s: &AppState, t: &Theme, body: &str) -> Option<El> {
    let section = s.section.last.as_ref()?;
    section.components.iter().find(|c| c.component_id == body)?;
    let note = |key: &str, msg: String, color| {
        text(msg)
            .key(key.to_owned())
            .role("status")
            .font(type_::BODY)
            .color(color)
    };
    let mut c = col().key("regions").w_full().gap(space::S1).child(
        text("Exact section regions")
            .font(type_::SECTION)
            .color(t.text_secondary)
            .pt(space::S2),
    );
    c = match s.section.regions.get(&section.section_id) {
        None | Some(Load::Loading) => c.child(note(
            "regions-pending",
            "Computing exact region metrics\u{2026}".into(),
            t.text_secondary,
        )),
        Some(Load::Failed(message)) => c.child(note(
            "regions-error",
            format!("Region metrics failed: {message}"),
            severity_color(Severity::Error, t),
        )),
        Some(Load::Ready(_)) => {
            let regions = s.section.regions.of_body(&section.section_id, body);
            if regions.is_empty() {
                return Some(c.child(note(
                    "regions-none",
                    "This body has no closed region in the section.".into(),
                    t.text_secondary,
                )));
            }
            let f = &s.shell.number_format;
            for r in regions {
                let n = r.region_index + 1;
                c = c
                    .child(text(format!("Region {n}")).font(type_::LABEL).color(t.text))
                    .child(
                        label_value(t, "Area", f.area(r.area_mm2)).key(format!("region-{n}-area")),
                    )
                    .child(label_value(t, "Perimeter", f.length(r.perimeter_mm)))
                    .child(label_value(
                        t,
                        "Centroid",
                        format!(
                            "({}{}{})",
                            f.length(r.centroid[0]),
                            f.list_separator(),
                            f.length(r.centroid[1])
                        ),
                    ))
                    .child(label_value(t, "Holes", r.hole_count.to_string()));
            }
            c
        }
    };
    Some(c)
}

#[cfg(test)]
mod tests {
    use rune::{Env, Runtime};

    use crate::api::dto::{
        RegionMetric, SectionComponent, SectionPlane, SectionResponse, SectionWire, WireRole,
    };
    use crate::state::AppState;
    use crate::state::model::tests::summary;
    use crate::state::section::SectionStats;

    fn runtime() -> Runtime<AppState> {
        let mut s = AppState::default();
        s.model.set_imported(summary(&["a"]));
        s.model.select(Some("a".into()));
        let section = SectionResponse {
            section_id: "s1".into(),
            import_id: "imp".into(),
            plane: SectionPlane::XY { offset_mm: 0.0 },
            components: vec![SectionComponent {
                component_id: "a".into(),
                wires: vec![SectionWire {
                    closed: true,
                    role: Some(WireRole::Outer),
                    curves: vec![],
                }],
            }],
            diagnostics: vec![],
        };
        s.section.stats = Some(SectionStats::of(&section));
        s.section.last = Some(section);
        let mut rt = Runtime::new(s, crate::ui::view, (1280.0, 760.0), Env::new(true, 1.0));
        rt.frame(0.0);
        rt.frame(1.0 / 60.0);
        rt
    }

    fn refresh(rt: &mut Runtime<AppState>) {
        rt.invalidate();
        rt.frame(1.0 / 60.0);
    }

    #[test]
    fn the_pane_shows_computing_then_exact_metrics_or_the_error() {
        let mut rt = runtime();
        assert!(
            rt.find("regions-pending").is_some(),
            "computing before the reply"
        );
        rt.state.section.regions.begin("s1");
        rt.state.section.regions.finish(
            "s1",
            Ok(vec![RegionMetric {
                component_id: "a".into(),
                region_index: 0,
                area_mm2: 96.5,
                perimeter_mm: 42.0,
                centroid: [5.0, 5.0],
                hole_count: 1,
            }]),
        );
        refresh(&mut rt);
        assert!(rt.find("regions-pending").is_none() && rt.find("region-1-area").is_some());

        let mut rt = runtime();
        rt.state.section.regions.begin("s1");
        rt.state
            .section
            .regions
            .finish("s1", Err("OCC failed".into()));
        refresh(&mut rt);
        assert!(rt.find("regions-error").is_some());
    }

    #[test]
    fn a_body_outside_the_shown_section_has_no_pane() {
        let rt = runtime();
        let t = rune::tok::theme(false);
        assert!(super::rows(&rt.state, &t, "zzz").is_none());
        assert!(super::rows(&rt.state, &t, "a").is_some());
    }
}
