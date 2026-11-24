//! Exact region metrics from the geometry service, fetched lazily and cached per section id.
//! Display only: the numbers come from OpenCascade, the UI never recomputes them.

use std::collections::HashMap;

use crate::api::dto::RegionMetric;

#[derive(Debug, Clone, PartialEq)]
pub enum Load {
    Loading,
    Ready(Vec<RegionMetric>),
    Failed(String),
}

#[derive(Debug, Default)]
pub struct RegionCache {
    by_section: HashMap<String, Load>,
}

impl RegionCache {
    pub fn get(&self, section_id: &str) -> Option<&Load> {
        self.by_section.get(section_id)
    }

    /// Marks the section as loading; false when it is already cached or in flight.
    pub fn begin(&mut self, section_id: &str) -> bool {
        if self.by_section.contains_key(section_id) {
            return false;
        }
        self.by_section.insert(section_id.to_owned(), Load::Loading);
        true
    }

    /// Forgets a failed fetch so the next `begin` requests it again; true when it was failed.
    pub fn retry(&mut self, section_id: &str) -> bool {
        let failed = matches!(self.by_section.get(section_id), Some(Load::Failed(_)));
        if failed {
            self.by_section.remove(section_id);
        }
        failed
    }

    pub fn finish(&mut self, section_id: &str, result: Result<Vec<RegionMetric>, String>) {
        let load = result.map_or_else(Load::Failed, Load::Ready);
        self.by_section.insert(section_id.to_owned(), load);
    }

    /// Regions of one body in a section, once loaded.
    pub fn of_body<'a>(&'a self, section_id: &str, body: &str) -> Vec<&'a RegionMetric> {
        match self.get(section_id) {
            Some(Load::Ready(all)) => all.iter().filter(|r| r.component_id == body).collect(),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metric(body: &str, index: usize) -> RegionMetric {
        RegionMetric {
            component_id: body.into(),
            region_index: index,
            area_mm2: 1.0,
            perimeter_mm: 4.0,
            centroid: [0.5, 0.5],
            hole_count: 0,
        }
    }

    #[test]
    fn a_section_is_requested_once_and_its_result_is_cached() {
        let mut c = RegionCache::default();
        assert!(c.begin("s1"));
        assert!(!c.begin("s1"), "already loading");
        assert_eq!(c.get("s1"), Some(&Load::Loading));
        c.finish(
            "s1",
            Ok(vec![metric("a", 0), metric("b", 0), metric("a", 1)]),
        );
        assert!(!c.begin("s1"), "cached");
        assert_eq!(c.of_body("s1", "a").len(), 2);
        assert!(c.of_body("s2", "a").is_empty());
        assert!(c.begin("s2"), "another section is another request");
    }

    #[test]
    fn only_a_failed_fetch_can_be_retried() {
        let mut c = RegionCache::default();
        assert!(!c.retry("s1"));
        c.begin("s1");
        assert!(!c.retry("s1"), "still loading");
        c.finish("s1", Err("boom".into()));
        assert!(c.retry("s1"));
        assert!(c.begin("s1"), "requested again");
        c.finish("s1", Ok(vec![]));
        assert!(!c.retry("s1"), "a loaded result is kept");
    }

    #[test]
    fn a_failure_is_kept_so_it_can_be_shown() {
        let mut c = RegionCache::default();
        c.begin("s1");
        c.finish("s1", Err("boom".into()));
        assert_eq!(c.get("s1"), Some(&Load::Failed("boom".into())));
        assert!(c.of_body("s1", "a").is_empty());
    }
}
