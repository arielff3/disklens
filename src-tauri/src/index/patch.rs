//! In-place updates for the live watcher. Every change applies a signed delta
//! to all ancestors and keeps each parent's children sorted. Removed nodes
//! become tombstones: unlinked from their parent, ids never reused.

use super::{dominant, DirTree, FileRec, Index, Node, NodeId, NO_PARENT};
use crate::categorize::{categorize, CATEGORY_COUNT};
use std::cmp::Ordering;

/// Signed change to a node's totals, applied to each of its ancestors.
#[derive(Default)]
struct Delta {
    size: i64,
    alloc: i64,
    files: i64,
    cats: [i64; CATEGORY_COUNT],
}

impl Delta {
    /// Everything a node contributes to its ancestors.
    fn of(node: &Node) -> Delta {
        let mut d = Delta { size: node.size as i64, alloc: node.alloc as i64, files: node.files as i64, ..Default::default() };
        match &node.cat_alloc {
            Some(cats) => {
                for (d, &c) in d.cats.iter_mut().zip(cats.iter()) {
                    *d = c as i64;
                }
            }
            None => d.cats[node.category.index()] = node.alloc as i64,
        }
        d
    }

    fn negated(mut self) -> Delta {
        self.size = -self.size;
        self.alloc = -self.alloc;
        self.files = -self.files;
        self.cats.iter_mut().for_each(|c| *c = -*c);
        self
    }
}

impl Index {
    /// Updates an existing file's sizes and modification time.
    pub fn set_file(&mut self, id: NodeId, rec: &FileRec) {
        let node = &mut self.nodes[id as usize];
        if node.is_dir || node.removed {
            return;
        }
        let mut delta = Delta {
            size: rec.size as i64 - node.size as i64,
            alloc: rec.alloc as i64 - node.alloc as i64,
            ..Default::default()
        };
        delta.cats[node.category.index()] = delta.alloc;
        node.size = rec.size;
        node.alloc = rec.alloc;
        node.modified = rec.modified;
        self.bubble(id, &delta);
    }

    /// Updates a folder's own modification time (its totals come from its children).
    pub fn set_modified(&mut self, id: NodeId, modified: i64) {
        if let Some(n) = self.nodes.get_mut(id as usize) {
            n.modified = modified;
        }
    }

    /// Adds a file to the folder `parent` and returns its id.
    pub fn add_file(&mut self, parent: NodeId, rec: FileRec) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(Node {
            category: categorize(&rec.name),
            name: rec.name.into_boxed_str(),
            parent,
            is_dir: false,
            size: rec.size,
            alloc: rec.alloc,
            files: 1,
            modified: rec.modified,
            unreadable: false,
            children: Vec::new(),
            cat_alloc: None,
            removed: false,
        });
        self.link(parent, id);
        id
    }

    /// Adds a freshly scanned folder (and everything in it) to `parent`.
    pub fn graft_dir(&mut self, parent: NodeId, tree: DirTree) -> NodeId {
        let id = self.add_dir(tree, parent);
        self.link(parent, id);
        id
    }

    /// Removes a node and everything below it. The root can't be removed.
    pub fn remove(&mut self, id: NodeId) {
        let Some(node) = self.nodes.get(id as usize) else { return };
        if node.removed || node.parent == NO_PARENT {
            return;
        }
        let parent = node.parent;
        let delta = Delta::of(node).negated();
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            let n = &mut self.nodes[cur as usize];
            n.removed = true;
            if n.unreadable {
                self.unreadable_dirs -= 1;
            }
            stack.extend(n.children.iter().copied());
        }
        self.nodes[parent as usize].children.retain(|&c| c != id);
        self.bubble(id, &delta);
    }

    /// Inserts a new child into its parent's sorted children and adds its totals upwards.
    fn link(&mut self, parent: NodeId, child: NodeId) {
        self.nodes[parent as usize].children.push(child);
        let delta = Delta::of(&self.nodes[child as usize]);
        self.bubble(child, &delta);
    }

    /// Applies `delta` to every ancestor of `child`, re-sorting each changed node
    /// inside its parent on the way up.
    fn bubble(&mut self, mut child: NodeId, delta: &Delta) {
        loop {
            let parent = self.nodes[child as usize].parent;
            if parent == NO_PARENT {
                break;
            }
            if !self.nodes[child as usize].removed {
                self.resort(parent, child);
            }
            let p = &mut self.nodes[parent as usize];
            p.size = p.size.saturating_add_signed(delta.size);
            p.alloc = p.alloc.saturating_add_signed(delta.alloc);
            p.files = p.files.saturating_add_signed(delta.files);
            if let Some(cats) = &mut p.cat_alloc {
                for (c, &d) in cats.iter_mut().zip(delta.cats.iter()) {
                    *c = c.saturating_add_signed(d);
                }
                p.category = dominant(cats);
            }
            child = parent;
        }
    }

    /// Moves `child` to its place in `parent`'s children (largest first, then by name).
    fn resort(&mut self, parent: NodeId, child: NodeId) {
        let mut children = std::mem::take(&mut self.nodes[parent as usize].children);
        if let Some(pos) = children.iter().position(|&c| c == child) {
            children.remove(pos);
        }
        let nodes = &self.nodes;
        let at = children
            .binary_search_by(|&c| child_order(&nodes[c as usize], &nodes[child as usize]))
            .unwrap_or_else(|i| i);
        children.insert(at, child);
        self.nodes[parent as usize].children = children;
    }
}

/// The order of a folder's children: largest `alloc` first, ties by name.
pub(super) fn child_order(a: &Node, b: &Node) -> Ordering {
    b.alloc.cmp(&a.alloc).then_with(|| a.name.cmp(&b.name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categorize::Category;
    use crate::index::NO_PARENT;

    fn file(name: &str, size: u64, alloc: u64) -> FileRec {
        FileRec { name: name.into(), size, alloc, modified: 0 }
    }

    fn dir(name: &str, files: Vec<FileRec>, dirs: Vec<DirTree>) -> DirTree {
        DirTree { name: name.into(), files, dirs, ..Default::default() }
    }

    /// C:\ { a.mp4 (4096), Users\ { b.jpg (4096), c.txt (8192), Docs\ { d.pdf (12288) } } }
    fn tree() -> DirTree {
        let docs = dir("Docs", vec![file("d.pdf", 12000, 12288)], vec![]);
        let users = dir("Users", vec![file("b.jpg", 10, 4096), file("c.txt", 5000, 8192)], vec![docs]);
        dir(r"C:\", vec![file("a.mp4", 100, 4096)], vec![users])
    }

    fn id_of(index: &Index, path: &str) -> NodeId {
        index.find_path(path).unwrap_or_else(|| panic!("{path} not found"))
    }

    fn assert_same(patched: &Index, expected: DirTree) {
        assert_eq!(patched.outline(), Index::build(expected).outline());
    }

    #[test]
    fn set_file_grows_and_shrinks_and_resorts() {
        let mut index = Index::build(tree());
        let b = id_of(&index, r"C:\Users\b.jpg");
        index.set_file(b, &file("b.jpg", 30000, 32768));
        let docs = dir("Docs", vec![file("d.pdf", 12000, 12288)], vec![]);
        let users = dir("Users", vec![file("b.jpg", 30000, 32768), file("c.txt", 5000, 8192)], vec![docs]);
        assert_same(&index, dir(r"C:\", vec![file("a.mp4", 100, 4096)], vec![users]));
        assert_eq!(index.node(id_of(&index, r"C:\Users")).unwrap().category, Category::Images);

        index.set_file(b, &file("b.jpg", 10, 4096));
        assert_same(&index, tree());
    }

    #[test]
    fn add_file_updates_ancestors_and_dominant_category() {
        let mut index = Index::build(tree());
        let docs = id_of(&index, r"C:\Users\Docs");
        let id = index.add_file(docs, file("e.mkv", 90000, 90112));
        assert_eq!(index.path_of(id), r"C:\Users\Docs\e.mkv");
        let docs = dir("Docs", vec![file("d.pdf", 12000, 12288), file("e.mkv", 90000, 90112)], vec![]);
        let users = dir("Users", vec![file("b.jpg", 10, 4096), file("c.txt", 5000, 8192)], vec![docs]);
        assert_same(&index, dir(r"C:\", vec![file("a.mp4", 100, 4096)], vec![users]));
        assert_eq!(index.node(0).unwrap().category, Category::Video);
    }

    #[test]
    fn graft_dir_adds_a_subtree() {
        let mut index = Index::build(tree());
        let locked = DirTree { name: "Locked".into(), unreadable: true, ..Default::default() };
        let new = dir("Games", vec![file("g.pak", 50000, 53248)], vec![locked]);
        let id = index.graft_dir(0, new);
        assert_eq!(index.path_of(id), r"C:\Games");
        let mut expected = tree();
        let locked = DirTree { name: "Locked".into(), unreadable: true, ..Default::default() };
        expected.dirs.push(dir("Games", vec![file("g.pak", 50000, 53248)], vec![locked]));
        assert_same(&index, expected);
        assert_eq!(index.unreadable_dirs(), 1);
        assert!(index.find_path(r"C:\Games\Locked").is_some());
    }

    #[test]
    fn remove_file_and_folder_leaves_tombstones() {
        let mut index = Index::build(tree());
        let c = id_of(&index, r"C:\Users\c.txt");
        index.remove(c);
        assert!(!index.is_live(c));
        assert!(index.view(c, 10).is_none());
        assert_eq!(index.find_path(r"C:\Users\c.txt"), None);

        let docs = id_of(&index, r"C:\Users\Docs");
        let pdf = id_of(&index, r"C:\Users\Docs\d.pdf");
        index.remove(docs);
        assert!(!index.is_live(docs) && !index.is_live(pdf));
        let users = dir("Users", vec![file("b.jpg", 10, 4096)], vec![]);
        assert_same(&index, dir(r"C:\", vec![file("a.mp4", 100, 4096)], vec![users]));
    }

    #[test]
    fn removing_an_unreadable_folder_updates_the_count() {
        let locked = DirTree { name: "Locked".into(), unreadable: true, ..Default::default() };
        let mut index = Index::build(dir(r"C:\", vec![], vec![dir("A", vec![], vec![locked])]));
        assert_eq!(index.unreadable_dirs(), 1);
        index.remove(id_of(&index, r"C:\A"));
        assert_eq!(index.unreadable_dirs(), 0);
    }

    #[test]
    fn root_cannot_be_removed() {
        let mut index = Index::build(tree());
        index.remove(0);
        assert!(index.is_live(0));
        assert_eq!(index.node(0).unwrap().parent, NO_PARENT);
        assert_same(&index, tree());
    }

    #[test]
    fn rename_as_remove_and_add_changes_category() {
        let mut index = Index::build(tree());
        let users = id_of(&index, r"C:\Users");
        index.remove(id_of(&index, r"C:\Users\c.txt"));
        index.add_file(users, file("c.mp3", 5000, 8192));
        let docs = dir("Docs", vec![file("d.pdf", 12000, 12288)], vec![]);
        let users = dir("Users", vec![file("b.jpg", 10, 4096), file("c.mp3", 5000, 8192)], vec![docs]);
        assert_same(&index, dir(r"C:\", vec![file("a.mp4", 100, 4096)], vec![users]));
    }

    #[test]
    fn nearest_returns_deepest_existing_ancestor() {
        let index = Index::build(tree());
        let users = id_of(&index, r"C:\Users");
        assert_eq!(index.nearest(r"C:\Users\gone\deeper"), Some(users));
        assert_eq!(index.nearest(r"C:\Users"), Some(users));
        assert_eq!(index.nearest(r"D:\x"), None);
    }
}
