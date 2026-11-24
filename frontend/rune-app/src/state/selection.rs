//! Tree navigation and multi-selection. `ModelState::selected` stays the single selection that
//! drives properties, 3D and 2D; `multi` only widens what visibility and grouping act on.

use super::model::{ModelState, group_key};
use super::tree::Folder;

/// A row of the model tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Model,
    Section,
    Bodies,
    Folder(String),
    Body(String),
    Groups,
    Group(String),
    /// A body listed under a group.
    Member(String, String),
}

impl Node {
    /// Key in `ModelState::collapsed` for nodes that expand and collapse.
    pub fn collapse_key(&self) -> Option<String> {
        match self {
            Node::Model => Some("model".into()),
            Node::Bodies => Some("bodies".into()),
            Node::Groups => Some("groups".into()),
            Node::Folder(key) => Some(key.clone()),
            Node::Group(name) => Some(group_key(name)),
            _ => None,
        }
    }

    pub fn body_id(&self) -> Option<&str> {
        match self {
            Node::Body(id) | Node::Member(_, id) => Some(id),
            _ => None,
        }
    }
}

/// A context menu opened at a window point on `node`.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenu {
    pub node: Node,
    pub x: f32,
    pub y: f32,
    pub sub_open: bool,
}

fn folder_nodes(f: &Folder, depth: usize, out: &mut Vec<(Node, usize)>, m: &ModelState) {
    for b in &f.bodies {
        out.push((Node::Body(b.id.clone()), depth));
    }
    for sub in &f.folders {
        out.push((Node::Folder(sub.key.clone()), depth));
        if !m.is_collapsed(&sub.key) {
            folder_nodes(sub, depth + 1, out, m);
        }
    }
}

impl ModelState {
    /// The visible tree rows with their depth, in display order (mirrors `ui::model_tree`).
    pub fn rows(&self) -> Vec<(Node, usize)> {
        let mut out = vec![];
        if self.imported.is_none() {
            return out;
        }
        out.push((Node::Model, 0));
        if self.is_collapsed("model") {
            return out;
        }
        out.push((Node::Section, 1));
        out.push((Node::Bodies, 1));
        if !self.is_collapsed("bodies") {
            folder_nodes(&self.visible_tree(), 2, &mut out, self);
        }
        if self.groups.is_empty() {
            return out;
        }
        out.push((Node::Groups, 1));
        if self.is_collapsed("groups") {
            return out;
        }
        for g in &self.groups {
            out.push((Node::Group(g.name.clone()), 2));
            if !self.is_collapsed(&group_key(&g.name)) {
                for b in self.visible_members(g) {
                    out.push((Node::Member(g.name.clone(), b.id.clone()), 3));
                }
            }
        }
        out
    }

    pub fn current(&self) -> Node {
        self.cursor.clone().unwrap_or(Node::Model)
    }

    /// Moves the cursor to `node`; selectable nodes also become the selection.
    pub fn activate(&mut self, node: &Node) {
        match node {
            Node::Model => self.select(None),
            Node::Section => self.select_section(),
            Node::Body(id) | Node::Member(_, id) => self.select(Some(id.clone())),
            Node::Group(name) => self.select_group(name),
            _ => {}
        }
        self.cursor = Some(node.clone());
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let at = rows
            .iter()
            .position(|(n, _)| *n == self.current())
            .unwrap_or(0);
        let to = at.saturating_add_signed(delta).min(rows.len() - 1);
        self.activate(&rows[to].0.clone());
    }

    fn parent(&self, node: &Node) -> Option<Node> {
        let rows = self.rows();
        let at = rows.iter().position(|(n, _)| n == node)?;
        let depth = rows[at].1;
        rows[..at]
            .iter()
            .rev()
            .find(|(_, d)| *d < depth)
            .map(|(n, _)| n.clone())
    }

    /// Left: collapse an open node, else go to the parent.
    pub fn collapse_or_parent(&mut self) {
        let node = self.current();
        match node.collapse_key() {
            Some(key) if !self.collapsed.contains(&key) && self.filter.text.trim().is_empty() => {
                self.collapsed.insert(key);
            }
            _ => {
                if let Some(parent) = self.parent(&node) {
                    self.activate(&parent);
                }
            }
        }
    }

    /// Right: expand a collapsed node, else go to its first child.
    pub fn expand_or_child(&mut self) {
        let node = self.current();
        if node
            .collapse_key()
            .is_some_and(|key| self.collapsed.remove(&key))
        {
            return;
        }
        let rows = self.rows();
        if let Some(at) = rows.iter().position(|(n, _)| *n == node)
            && let Some((next, depth)) = rows.get(at + 1)
            && *depth > rows[at].1
        {
            self.activate(&next.clone());
        }
    }

    /// A body row was clicked; Ctrl toggles it in the multi-selection, Shift extends a range.
    pub fn click_body(&mut self, node: &Node, ctrl: bool, shift: bool) {
        let Some(id) = node.body_id().map(str::to_owned) else {
            return;
        };
        if !(ctrl || shift) {
            self.activate(node);
            self.anchor = Some(id);
            return;
        }
        if shift {
            let mut order: Vec<String> = vec![];
            for (n, _) in self.rows() {
                if let Some(b) = n.body_id()
                    && !order.iter().any(|o| o == b)
                {
                    order.push(b.to_owned());
                }
            }
            let anchor = self.anchor.clone().or(self.selected.clone());
            let a = anchor.and_then(|a| order.iter().position(|o| *o == a));
            let b = order.iter().position(|o| *o == id);
            if let (Some(a), Some(b)) = (a, b) {
                self.multi = order[a.min(b)..=a.max(b)].iter().cloned().collect();
            }
        } else {
            if self.multi.is_empty()
                && let Some(prev) = self.selected.clone()
            {
                self.multi.insert(prev);
            }
            if !self.multi.remove(&id) {
                self.multi.insert(id.clone());
            }
            self.anchor = Some(id.clone());
        }
        self.selected = if self.multi.contains(&id) {
            Some(id)
        } else {
            self.multi.iter().next_back().cloned()
        };
        self.related = None;
        self.selected_group = None;
        self.section_selected = false;
        self.cursor = Some(node.clone());
    }

    /// Bodies that visibility and grouping act on: the multi-selection, else the selection.
    pub fn targets(&self) -> Vec<String> {
        if self.multi.len() > 1 {
            self.multi.iter().cloned().collect()
        } else {
            self.selected.iter().cloned().collect()
        }
    }

    /// Whether `id` is part of the picked bodies (for row highlight).
    pub fn is_picked(&self, id: &str) -> bool {
        self.selected.as_deref() == Some(id) || self.multi.contains(id)
    }

    /// Hides all `ids`, or shows them when they are all hidden already.
    pub fn toggle_hidden_many(&mut self, ids: &[String]) {
        let all = !ids.is_empty() && ids.iter().all(|i| self.hidden.contains(i));
        self.isolated = false;
        for id in ids {
            if all {
                self.hidden.remove(id);
            } else {
                self.hidden.insert(id.clone());
            }
        }
    }

    pub fn add_to_group(&mut self, group: &str, ids: &[String]) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.name == group) {
            for id in ids {
                if !g.component_ids.contains(id) {
                    g.component_ids.push(id.clone());
                }
            }
        }
    }

    /// Expands every node.
    pub fn expand_all(&mut self) {
        self.collapsed.clear();
    }

    /// Collapses every folder, group, Bodies and Groups (the model node stays open).
    pub fn collapse_all(&mut self) {
        fn folders(f: &Folder, out: &mut Vec<String>) {
            for sub in &f.folders {
                out.push(sub.key.clone());
                folders(sub, out);
            }
        }
        let mut keys = vec!["bodies".to_owned(), "groups".to_owned()];
        if let Some(i) = &self.imported {
            folders(
                &super::tree::skip_shared(super::tree::build(&i.components)),
                &mut keys,
            );
        }
        keys.extend(self.groups.iter().map(|g| group_key(&g.name)));
        self.collapsed.extend(keys);
    }

    /// Hover from a viewport; returns whether the tree should scroll to the row.
    pub fn hover_body(&mut self, id: Option<String>) -> bool {
        let changed = self.hover != id;
        self.hover = id;
        changed && self.hover.is_some()
    }

    /// F2 on a group: edit its name in place.
    pub fn start_rename(&mut self, name: &str) {
        self.activate(&Node::Group(name.to_owned()));
        self.renaming = true;
        self.rename_group.focused = true;
    }

    pub fn commit_rename(&mut self) -> Result<(), String> {
        let result = self.rename_selected_group();
        if result.is_ok() {
            self.renaming = false;
            if let Some(name) = self.selected_group.clone() {
                self.cursor = Some(Node::Group(name));
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::model::BodyGroup;
    use crate::state::model::tests::{body, summary};

    fn model() -> ModelState {
        let mut m = ModelState::default();
        let mut s = summary(&[]);
        s.components = vec![
            body("a", "A", &["Top", "A"]),
            body("b", "B", &["Top", "Sub", "B"]),
            body("c", "C", &["Top", "Sub", "C"]),
        ];
        s.component_count = 3;
        m.set_imported(s);
        m
    }

    #[test]
    fn rows_follow_the_tree_and_arrows_walk_them() {
        let mut m = model();
        let kinds: Vec<Node> = m.rows().into_iter().map(|r| r.0).collect();
        assert_eq!(
            kinds,
            [
                Node::Model,
                Node::Section,
                Node::Bodies,
                Node::Body("a".into()),
                Node::Folder("Top / Sub".into()),
                Node::Body("b".into()),
                Node::Body("c".into())
            ]
        );
        assert_eq!(m.current(), Node::Section);
        m.move_cursor(2);
        assert_eq!(m.current(), Node::Body("a".into()));
        assert_eq!(m.selected.as_deref(), Some("a"));
        m.move_cursor(1); // a folder: cursor only, selection stays
        assert_eq!(m.current(), Node::Folder("Top / Sub".into()));
        assert_eq!(m.selected.as_deref(), Some("a"));
        m.collapse_or_parent();
        assert!(m.collapsed.contains("Top / Sub"));
        m.expand_or_child();
        m.expand_or_child();
        assert_eq!(m.current(), Node::Body("b".into()));
        m.collapse_or_parent();
        assert_eq!(m.current(), Node::Folder("Top / Sub".into()));
        m.move_cursor(-100);
        assert_eq!(m.current(), Node::Model);
    }

    #[test]
    fn collapse_all_and_expand_all_cover_folders_and_groups() {
        let mut m = model();
        m.groups.push(BodyGroup {
            name: "G".into(),
            component_ids: vec!["a".into()],
        });
        m.collapse_all();
        let kinds: Vec<Node> = m.rows().into_iter().map(|r| r.0).collect();
        assert_eq!(
            kinds,
            [Node::Model, Node::Section, Node::Bodies, Node::Groups]
        );
        m.expand_all();
        assert!(m.rows().len() > 6);
        assert!(m.hover_body(Some("a".into())) && !m.hover_body(Some("a".into())));
        assert!(!m.hover_body(None));
    }

    #[test]
    fn ctrl_and_shift_build_a_multi_selection_while_selected_stays_single() {
        let mut m = model();
        m.click_body(&Node::Body("a".into()), false, false);
        m.click_body(&Node::Body("c".into()), true, false);
        assert_eq!(m.targets(), ["a", "c"]);
        assert_eq!(m.selected.as_deref(), Some("c"));
        m.click_body(&Node::Body("c".into()), true, false);
        assert_eq!(m.selected.as_deref(), Some("a"));
        assert_eq!(m.targets(), ["a"]);
        m.click_body(&Node::Body("a".into()), false, false);
        m.click_body(&Node::Body("c".into()), false, true);
        assert_eq!(m.targets(), ["a", "b", "c"]);
        m.toggle_hidden_many(&m.targets());
        assert_eq!(m.hidden.len(), 3);
        m.toggle_hidden_many(&m.targets());
        assert!(m.hidden.is_empty());
        m.groups.push(BodyGroup {
            name: "G".into(),
            component_ids: vec!["a".into()],
        });
        m.add_to_group("G", &m.targets());
        assert_eq!(m.groups[0].component_ids, ["a", "b", "c"]);
        m.select(Some("b".into()));
        assert_eq!(
            m.targets(),
            ["b"],
            "a plain select drops the multi-selection"
        );
    }
}
