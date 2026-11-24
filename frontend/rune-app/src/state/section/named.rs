//! Named sections of a project: each keeps its plane, offset, tolerance and the path it was last
//! exported to. The live plane and tolerance fields of `SectionState` are the active section's
//! working copy; they are written back when the section is left, saved or listed.

use rune::TextEdit;

use super::{PlaneSnapshot, SectionState};
use crate::api::dto::Tolerance;

pub const NAME_MAX: usize = 60;

#[derive(Debug, Clone, PartialEq)]
pub struct NamedSection {
    pub name: String,
    pub plane: PlaneSnapshot,
    pub tolerance: Tolerance,
    pub last_export: Option<String>,
}

#[derive(Debug, Default)]
pub struct Sections {
    pub list: Vec<NamedSection>,
    pub active: usize,
    /// Bound to the name field of the section pane.
    pub name_edit: TextEdit,
    /// Why the last add or rename was refused.
    pub error: Option<String>,
}

impl SectionState {
    fn live(&self, name: String, last_export: Option<String>) -> NamedSection {
        NamedSection {
            name,
            plane: self.snapshot(),
            tolerance: self.tolerance.value,
            last_export,
        }
    }

    /// Makes sure at least one section exists (the one being edited).
    pub fn ensure_named(&mut self) {
        if self.named.list.is_empty() {
            let first = self.live("Section 1".into(), None);
            self.named.list.push(first);
            self.named.active = 0;
            self.named.name_edit.replace_text("Section 1");
        }
    }

    /// The sections with the live plane and tolerance merged into the active one.
    pub fn sections_now(&self) -> Vec<NamedSection> {
        let mut list = self.named.list.clone();
        if let Some(active) = list.get_mut(self.named.active) {
            active.plane = self.snapshot();
            active.tolerance = self.tolerance.value;
        }
        list
    }

    fn store_active(&mut self) {
        self.named.list = self.sections_now();
    }

    pub fn active_name(&self) -> Option<&str> {
        self.named
            .list
            .get(self.named.active)
            .map(|s| s.name.as_str())
    }

    fn check_name(&self, name: &str, except: Option<usize>) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("A section needs a name.".into());
        }
        if name.chars().count() > NAME_MAX {
            return Err(format!("Section names are at most {NAME_MAX} characters."));
        }
        let taken = self
            .named
            .list
            .iter()
            .enumerate()
            .any(|(i, s)| Some(i) != except && s.name.to_lowercase() == name.to_lowercase());
        if taken {
            return Err(format!("There is already a section named \"{name}\"."));
        }
        Ok(name.to_owned())
    }

    fn unique_name(&self) -> String {
        (1..)
            .map(|n| format!("Section {n}"))
            .find(|name| self.check_name(name, None).is_ok())
            .unwrap_or_default()
    }

    /// Adds a section starting from the current plane and switches to it.
    pub fn add_named(&mut self, name: &str) -> Result<(), String> {
        self.ensure_named();
        let typed = name.trim();
        let name = if typed.is_empty() || typed == self.active_name().unwrap_or_default() {
            self.unique_name()
        } else {
            self.check_name(typed, None)?
        };
        self.store_active();
        let added = self.live(name.clone(), None);
        self.named.list.push(added);
        self.named.active = self.named.list.len() - 1;
        self.last = None;
        self.stats = None;
        self.named.name_edit.replace_text(name);
        Ok(())
    }

    pub fn switch_to(&mut self, index: usize) {
        self.ensure_named();
        if index >= self.named.list.len() || index == self.named.active {
            return;
        }
        self.store_active();
        self.named.active = index;
        let target = self.named.list[index].clone();
        self.restore(target.plane);
        self.tolerance.restore(target.tolerance);
        self.named.name_edit.replace_text(target.name);
    }

    pub fn rename_active(&mut self, name: &str) -> Result<(), String> {
        self.ensure_named();
        let name = self.check_name(name, Some(self.named.active))?;
        self.named.list[self.named.active].name = name;
        Ok(())
    }

    /// Deletes a section; the last one cannot be deleted.
    pub fn delete_named(&mut self, index: usize) -> Result<(), String> {
        if self.named.list.len() < 2 {
            return Err("A project keeps at least one section.".into());
        }
        if index >= self.named.list.len() {
            return Ok(());
        }
        self.store_active();
        let was_active = index == self.named.active;
        self.named.list.remove(index);
        if index < self.named.active {
            self.named.active -= 1;
        }
        self.named.active = self.named.active.min(self.named.list.len() - 1);
        if was_active {
            let target = self.named.list[self.named.active].clone();
            self.restore(target.plane);
            self.tolerance.restore(target.tolerance);
            self.named.name_edit.replace_text(target.name);
        }
        Ok(())
    }

    pub fn set_last_export(&mut self, path: String) {
        self.ensure_named();
        let active = self.named.active;
        self.named.list[active].last_export = Some(path);
    }

    /// Replaces the list when a project opens; the active one is loaded into the live fields.
    pub fn load_named(&mut self, list: Vec<NamedSection>, active: usize) {
        if list.is_empty() {
            self.named = Sections::default();
            self.ensure_named();
            return;
        }
        let active = active.min(list.len() - 1);
        let target = list[active].clone();
        self.named = Sections {
            list,
            active,
            name_edit: TextEdit::new(target.name),
            error: None,
        };
        self.restore(target.plane);
        self.tolerance.restore(target.tolerance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::section::PlaneKind;

    #[test]
    fn sections_keep_their_own_plane_offset_and_tolerance() {
        let mut s = SectionState::default();
        s.set_offset(5.0, 0);
        s.add_named("").unwrap();
        assert_eq!(s.active_name(), Some("Section 2"));
        s.set_plane(PlaneKind::XZ, 10_000);
        s.set_offset(9.0, 20_000);
        s.tolerance.restore(Tolerance {
            join_mm: Some(2e-5),
            ..Default::default()
        });
        s.switch_to(0);
        assert_eq!((s.plane, s.offset_mm), (PlaneKind::XY, Some(5.0)));
        assert!(s.tolerance.is_default());
        s.switch_to(1);
        assert_eq!((s.plane, s.offset_mm), (PlaneKind::XZ, Some(9.0)));
        assert_eq!(s.tolerance.value.join_mm, Some(2e-5));
    }

    #[test]
    fn names_are_unique_non_empty_and_the_last_section_stays() {
        let mut s = SectionState::default();
        s.ensure_named();
        s.add_named("Mid stack").unwrap();
        assert!(s.add_named("mid stack").is_err());
        assert!(s.rename_active("Section 1").is_err());
        assert!(s.rename_active("  ").is_err());
        s.rename_active("End winding").unwrap();
        s.delete_named(1).unwrap();
        assert_eq!(s.named.list.len(), 1);
        assert!(s.delete_named(0).is_err());
        assert_eq!(s.active_name(), Some("Section 1"));
    }

    #[test]
    fn deleting_before_the_active_one_keeps_it_active_and_export_paths_are_per_section() {
        let mut s = SectionState::default();
        s.ensure_named();
        s.set_last_export("a.zip".into());
        s.add_named("B").unwrap();
        s.add_named("C").unwrap();
        s.set_last_export("c.zip".into());
        s.delete_named(0).unwrap();
        assert_eq!(s.active_name(), Some("C"));
        assert_eq!(
            s.named.list[s.named.active].last_export.as_deref(),
            Some("c.zip")
        );
        assert_eq!(s.sections_now()[0].last_export, None);
    }
}
