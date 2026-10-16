//! In-memory size tree built from a scan. Nodes live in one arena `Vec`;
//! directory sizes and per-category totals are aggregated bottom-up.

use crate::categorize::{categorize, Category, CATEGORY_COUNT};
use serde::{Deserialize, Serialize};

mod patch;

pub type NodeId = u32;
pub const NO_PARENT: NodeId = NodeId::MAX;

/// What a scanner produces for one directory and everything below it.
#[derive(Debug, Default)]
pub struct DirTree {
    pub name: String,
    pub modified: i64,
    /// The directory itself could not be listed (usually access denied).
    pub unreadable: bool,
    pub files: Vec<FileRec>,
    pub dirs: Vec<DirTree>,
}

#[derive(Debug, Clone)]
pub struct FileRec {
    pub name: String,
    /// Logical size in bytes.
    pub size: u64,
    /// Bytes actually allocated on disk.
    pub alloc: u64,
    pub modified: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub name: Box<str>,
    pub parent: NodeId,
    pub is_dir: bool,
    /// A file's category, or a directory's dominant category.
    pub category: Category,
    pub size: u64,
    pub alloc: u64,
    pub files: u64,
    pub modified: i64,
    pub unreadable: bool,
    /// Sorted by `alloc`, largest first.
    pub children: Vec<NodeId>,
    /// Allocated bytes per category, directories only.
    pub cat_alloc: Option<Box<[u64; CATEGORY_COUNT]>>,
    /// Deleted by a live update; kept as a tombstone so ids stay stable.
    pub removed: bool,
}

#[derive(Debug)]
pub struct Index {
    nodes: Vec<Node>,
    unreadable_dirs: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeView {
    pub id: NodeId,
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub alloc: u64,
    pub files: u64,
    pub modified: i64,
    pub unreadable: bool,
    /// Non-empty categories, largest first.
    pub categories: Vec<CategorySize>,
    /// From the scan root down to this node.
    pub crumbs: Vec<Crumb>,
    pub children: Vec<ChildView>,
    /// Children beyond the cap, summarized.
    pub rest: Option<RestView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySize {
    pub category: Category,
    pub alloc: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Crumb {
    pub id: NodeId,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildView {
    pub id: NodeId,
    pub name: String,
    pub is_dir: bool,
    pub alloc: u64,
    pub size: u64,
    pub files: u64,
    pub category: Category,
    pub unreadable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestView {
    pub count: u64,
    pub alloc: u64,
}

impl Index {
    pub fn build(root: DirTree) -> Index {
        let mut index = Index { nodes: Vec::new(), unreadable_dirs: 0 };
        index.add_dir(root, NO_PARENT);
        index
    }

    /// Rebuilds an index from saved nodes. Rejects node lists whose links don't
    /// form a tree rooted at node 0.
    pub fn from_parts(nodes: Vec<Node>, unreadable_dirs: u64) -> Option<Index> {
        let root = nodes.first()?;
        if root.parent != NO_PARENT || !root.is_dir {
            return None;
        }
        // Every node must be reached exactly once from the root, through a
        // parent that it points back to.
        let mut seen = vec![false; nodes.len()];
        seen[0] = true;
        let mut stack = vec![0usize];
        while let Some(id) = stack.pop() {
            for &c in nodes[id].children.iter() {
                let c = c as usize;
                if c >= nodes.len() || seen[c] || nodes[c].parent as usize != id {
                    return None;
                }
                seen[c] = true;
                stack.push(c);
            }
        }
        if seen.iter().any(|s| !s) {
            return None;
        }
        Some(Index { nodes, unreadable_dirs })
    }

    /// The live nodes renumbered in depth-first order, without tombstones.
    /// Borrows the arena as-is when there is nothing to drop.
    pub fn compacted(&self) -> std::borrow::Cow<'_, [Node]> {
        if !self.nodes.iter().any(|n| n.removed) {
            return std::borrow::Cow::Borrowed(&self.nodes);
        }
        let mut order = Vec::with_capacity(self.nodes.len());
        let mut stack = vec![self.root()];
        while let Some(id) = stack.pop() {
            order.push(id);
            stack.extend(self.nodes[id as usize].children.iter().rev());
        }
        let mut new_id = vec![NO_PARENT; self.nodes.len()];
        for (new, &old) in order.iter().enumerate() {
            new_id[old as usize] = new as NodeId;
        }
        let remap = |id: NodeId| if id == NO_PARENT { NO_PARENT } else { new_id[id as usize] };
        let nodes = order
            .iter()
            .map(|&old| {
                let n = &self.nodes[old as usize];
                Node {
                    parent: remap(n.parent),
                    children: n.children.iter().map(|&c| remap(c)).collect(),
                    ..n.clone()
                }
            })
            .collect();
        std::borrow::Cow::Owned(nodes)
    }

    pub fn root(&self) -> NodeId {
        0
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id as usize)
    }

    /// True for nodes that exist and haven't been removed by a live update.
    pub fn is_live(&self, id: NodeId) -> bool {
        self.node(id).is_some_and(|n| !n.removed)
    }

    /// Every node, tombstones included (check `removed`).
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn unreadable_dirs(&self) -> u64 {
        self.unreadable_dirs
    }

    fn add_dir(&mut self, dir: DirTree, parent: NodeId) -> NodeId {
        let id = self.nodes.len() as NodeId;
        if dir.unreadable {
            self.unreadable_dirs += 1;
        }
        self.nodes.push(Node {
            name: dir.name.into_boxed_str(),
            parent,
            is_dir: true,
            category: Category::Other,
            size: 0,
            alloc: 0,
            files: 0,
            modified: dir.modified,
            unreadable: dir.unreadable,
            children: Vec::new(),
            cat_alloc: None,
            removed: false,
        });

        let (mut size, mut alloc, mut files) = (0u64, 0u64, 0u64);
        let mut cats = [0u64; CATEGORY_COUNT];
        let mut children = Vec::with_capacity(dir.files.len() + dir.dirs.len());

        for f in dir.files {
            let category = categorize(&f.name);
            size += f.size;
            alloc += f.alloc;
            files += 1;
            cats[category.index()] += f.alloc;
            children.push(self.nodes.len() as NodeId);
            self.nodes.push(Node {
                name: f.name.into_boxed_str(),
                parent: id,
                is_dir: false,
                category,
                size: f.size,
                alloc: f.alloc,
                files: 1,
                modified: f.modified,
                unreadable: false,
                children: Vec::new(),
                cat_alloc: None,
                removed: false,
            });
        }
        for d in dir.dirs {
            let child = self.add_dir(d, id);
            let c = &self.nodes[child as usize];
            size += c.size;
            alloc += c.alloc;
            files += c.files;
            if let Some(child_cats) = &c.cat_alloc {
                for (total, v) in cats.iter_mut().zip(child_cats.iter()) {
                    *total += v;
                }
            }
            children.push(child);
        }

        let nodes = &self.nodes;
        children.sort_by(|&a, &b| patch::child_order(&nodes[a as usize], &nodes[b as usize]));

        let node = &mut self.nodes[id as usize];
        node.size = size;
        node.alloc = alloc;
        node.files = files;
        node.category = dominant(&cats);
        node.children = children;
        node.cat_alloc = Some(Box::new(cats));
        id
    }

    /// Full path of a node, e.g. `C:\Users\you`.
    pub fn path_of(&self, id: NodeId) -> String {
        let mut parts = Vec::new();
        let mut cur = id;
        while cur != NO_PARENT {
            let node = &self.nodes[cur as usize];
            parts.push(&*node.name);
            cur = node.parent;
        }
        let mut path = String::new();
        for part in parts.iter().rev() {
            if !path.is_empty() && !path.ends_with('\\') {
                path.push('\\');
            }
            path.push_str(part);
        }
        path
    }

    /// Finds a node by absolute path. Case-insensitive; accepts `/` or `\`.
    pub fn find_path(&self, path: &str) -> Option<NodeId> {
        self.locate(path).and_then(|(id, missing)| missing.is_none().then_some(id))
    }

    /// The node at `path`, or its deepest ancestor in the index.
    /// `None` if the path is outside the scanned root.
    pub fn nearest(&self, path: &str) -> Option<NodeId> {
        self.locate(path).map(|(id, _)| id)
    }

    /// Follows `path` down from the root as far as the index goes. Returns the
    /// deepest node reached and the first path component it has no child for
    /// (`None` when the whole path was found).
    pub fn locate(&self, path: &str) -> Option<(NodeId, Option<String>)> {
        let want = normalize(path);
        let root = normalize(&self.nodes.first()?.name);
        let head = want.get(..root.len())?;
        if !same_name(head, &root) {
            return None;
        }
        let rest = &want[root.len()..];
        if !rest.is_empty() && !root.ends_with('\\') && !rest.starts_with('\\') {
            return None;
        }
        let mut cur = self.root();
        for part in rest.split('\\').filter(|p| !p.is_empty()) {
            let next = self.nodes[cur as usize]
                .children
                .iter()
                .find(|&&c| same_name(&self.nodes[c as usize].name, part));
            match next {
                Some(&c) => cur = c,
                None => return Some((cur, Some(part.to_string()))),
            }
        }
        Some((cur, None))
    }

    /// One line per live node from the root down, in children order, with its
    /// totals. Two indexes with the same outline hold the same tree.
    pub fn outline(&self) -> Vec<String> {
        let mut out = vec![format!("unreadable={}", self.unreadable_dirs)];
        let mut stack = vec![self.root()];
        while let Some(id) = stack.pop() {
            let Some(n) = self.node(id) else { break };
            out.push(format!(
                "{} dir={} size={} alloc={} files={} modified={} unreadable={} cat={:?} cats={:?}",
                self.path_of(id),
                n.is_dir,
                n.size,
                n.alloc,
                n.files,
                n.modified,
                n.unreadable,
                n.category,
                n.cat_alloc
            ));
            stack.extend(n.children.iter().rev());
        }
        out
    }

    /// One node plus up to `max_children` children, ready for the UI.
    pub fn view(&self, id: NodeId, max_children: usize) -> Option<NodeView> {
        let n = self.node(id).filter(|n| !n.removed)?;
        let categories = match &n.cat_alloc {
            Some(cats) => {
                let mut v: Vec<CategorySize> = Category::ALL
                    .iter()
                    .filter(|c| cats[c.index()] > 0)
                    .map(|&c| CategorySize { category: c, alloc: cats[c.index()] })
                    .collect();
                // Stable sort: ties keep the `Category::ALL` order.
                v.sort_by_key(|c| std::cmp::Reverse(c.alloc));
                v
            }
            None => vec![CategorySize { category: n.category, alloc: n.alloc }],
        };
        let children = n.children.iter().take(max_children).map(|&c| self.child_view(c)).collect();
        let rest = (n.children.len() > max_children).then(|| {
            let others = &n.children[max_children..];
            RestView {
                count: others.len() as u64,
                alloc: others.iter().map(|&c| self.nodes[c as usize].alloc).sum(),
            }
        });
        Some(NodeView {
            id,
            name: n.name.to_string(),
            path: self.path_of(id),
            is_dir: n.is_dir,
            size: n.size,
            alloc: n.alloc,
            files: n.files,
            modified: n.modified,
            unreadable: n.unreadable,
            categories,
            crumbs: self.crumbs(id),
            children,
            rest,
        })
    }

    fn child_view(&self, id: NodeId) -> ChildView {
        let n = &self.nodes[id as usize];
        ChildView {
            id,
            name: n.name.to_string(),
            is_dir: n.is_dir,
            alloc: n.alloc,
            size: n.size,
            files: n.files,
            category: n.category,
            unreadable: n.unreadable,
        }
    }

    fn crumbs(&self, id: NodeId) -> Vec<Crumb> {
        let mut crumbs = Vec::new();
        let mut cur = id;
        while cur != NO_PARENT {
            let n = &self.nodes[cur as usize];
            crumbs.push(Crumb { id: cur, name: n.name.to_string() });
            cur = n.parent;
        }
        crumbs.reverse();
        crumbs
    }
}

/// Largest non-empty category; ties go to the earlier one in `Category::ALL`.
fn dominant(cats: &[u64; CATEGORY_COUNT]) -> Category {
    Category::ALL
        .iter()
        .copied()
        .filter(|c| cats[c.index()] > 0)
        .max_by(|a, b| cats[a.index()].cmp(&cats[b.index()]).then(b.index().cmp(&a.index())))
        .unwrap_or(Category::Other)
}

fn normalize(path: &str) -> String {
    let mut s = path.trim().replace('/', "\\");
    while s.len() > 3 && s.ends_with('\\') {
        s.pop();
    }
    s
}

fn same_name(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b) || a.to_lowercase() == b.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, size: u64, alloc: u64) -> FileRec {
        FileRec { name: name.into(), size, alloc, modified: 0 }
    }

    fn dir(name: &str, files: Vec<FileRec>, dirs: Vec<DirTree>) -> DirTree {
        DirTree { name: name.into(), files, dirs, ..Default::default() }
    }

    /// C:\ { a.mp4, Users\ { b.jpg, c.txt, Locked\ (unreadable) } }
    fn sample() -> Index {
        let locked = DirTree { name: "Locked".into(), unreadable: true, ..Default::default() };
        let users = dir("Users", vec![file("b.jpg", 10, 4096), file("c.txt", 5000, 8192)], vec![locked]);
        Index::build(dir(r"C:\", vec![file("a.mp4", 100, 4096)], vec![users]))
    }

    fn id_of(index: &Index, path: &str) -> NodeId {
        index.find_path(path).unwrap_or_else(|| panic!("{path} not found"))
    }

    fn child_names(index: &Index, id: NodeId) -> Vec<&str> {
        index.node(id).unwrap().children.iter().map(|&c| &*index.node(c).unwrap().name).collect()
    }

    #[test]
    fn aggregates_sizes_bottom_up() {
        let index = sample();
        let root = index.node(index.root()).unwrap();
        assert_eq!((root.size, root.alloc, root.files), (5110, 16384, 3));
        let users = index.node(id_of(&index, r"C:\Users")).unwrap();
        assert_eq!((users.size, users.alloc, users.files), (5010, 12288, 2));
    }

    #[test]
    fn sorts_children_largest_first() {
        let index = sample();
        assert_eq!(child_names(&index, 0), ["Users", "a.mp4"]);
        assert_eq!(child_names(&index, id_of(&index, r"C:\Users")), ["c.txt", "b.jpg", "Locked"]);
    }

    #[test]
    fn totals_categories_and_picks_dominant() {
        let index = sample();
        let view = index.view(0, 100).unwrap();
        let cats: Vec<_> = view.categories.iter().map(|c| (c.category, c.alloc)).collect();
        assert_eq!(
            cats,
            [(Category::Documents, 8192), (Category::Video, 4096), (Category::Images, 4096)]
        );
        assert_eq!(view.children[0].name, "Users");
        assert_eq!(view.children[0].category, Category::Documents);
    }

    #[test]
    fn builds_paths_and_finds_them_case_insensitively() {
        let index = sample();
        let c = id_of(&index, r"c:\users\C.TXT");
        assert_eq!(index.path_of(c), r"C:\Users\c.txt");
        assert_eq!(index.find_path(r"C:\"), Some(0));
        assert_eq!(index.find_path("C:/Users/"), index.find_path(r"C:\Users"));
        assert!(index.find_path(r"C:\Users").is_some());
        assert_eq!(index.find_path(r"D:\Users"), None);
        assert_eq!(index.find_path(r"C:\Nope"), None);
    }

    #[test]
    fn finds_paths_under_a_folder_root() {
        let index = Index::build(dir(r"C:\Users\you\Downloads", vec![file("x.zip", 1, 4096)], vec![]));
        assert_eq!(index.find_path(r"C:\Users\you\Downloads\x.zip"), Some(1));
        assert_eq!(index.find_path(r"C:\Users\you\DownloadsX"), None);
    }

    #[test]
    fn counts_unreadable_dirs() {
        assert_eq!(sample().unreadable_dirs(), 1);
    }

    #[test]
    fn view_caps_children_and_reports_the_rest() {
        let index = sample();
        let view = index.view(0, 1).unwrap();
        assert_eq!(view.children.len(), 1);
        assert_eq!(view.children[0].name, "Users");
        let rest = view.rest.unwrap();
        assert_eq!((rest.count, rest.alloc), (1, 4096));
    }

    #[test]
    fn view_has_breadcrumbs_from_root() {
        let index = sample();
        let view = index.view(id_of(&index, r"C:\Users\c.txt"), 10).unwrap();
        let names: Vec<_> = view.crumbs.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, [r"C:\", "Users", "c.txt"]);
        assert_eq!(view.path, r"C:\Users\c.txt");
        assert!(!view.is_dir);
    }

    #[test]
    fn from_parts_rejects_nodes_that_are_not_a_tree() {
        let good = sample().compacted().into_owned();
        assert!(Index::from_parts(good.clone(), 1).is_some());
        assert!(Index::from_parts(Vec::new(), 0).is_none());

        let mut dangling = good.clone();
        dangling[0].children.push(999);
        assert!(Index::from_parts(dangling, 0).is_none());

        // Two nodes pointing at each other, unreachable from the root.
        let mut cycle = good.clone();
        let (a, b) = (cycle.len() as NodeId, cycle.len() as NodeId + 1);
        let mut extra = cycle[1].clone();
        extra.parent = b;
        extra.children = vec![b];
        cycle.push(extra.clone());
        extra.parent = a;
        extra.children = vec![a];
        cycle.push(extra);
        assert!(Index::from_parts(cycle, 0).is_none());
    }
}
