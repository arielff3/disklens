//! Search and Biggest: run a [`Filter`] over the index in parallel and return
//! the largest matches.

pub mod query;

pub use query::{parse, Filter, ParseError};

use crate::categorize::Category;
use crate::index::{Index, Node, NodeId, NO_PARENT};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;

const DAY: i64 = 86_400;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub id: NodeId,
    pub name: String,
    /// Path of the folder that contains it.
    pub parent: String,
    pub is_dir: bool,
    pub alloc: u64,
    pub size: u64,
    pub modified: i64,
    pub category: Category,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// Largest first, at most `limit`.
    pub results: Vec<Hit>,
    /// Every match, including those past `limit`.
    pub total: u64,
    pub elapsed_ms: u64,
}

/// The Biggest tab's chips.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiggestFilter {
    pub min_size: Option<u64>,
    pub older_than_days: Option<u32>,
    pub category: Option<Category>,
}

pub fn biggest_filter(chips: &BiggestFilter) -> Filter {
    Filter {
        min_alloc: chips.min_size,
        older_than: chips.older_than_days.map(|d| d as i64 * DAY),
        kind: chips.category,
        files_only: true,
        ..Default::default()
    }
}

/// Runs `filter` over every live node. `places` maps lowercase place keys
/// (`downloads`) to paths for `in:`; `now` is Unix seconds.
pub fn run(index: &Index, filter: &Filter, places: &[(String, String)], now: i64, limit: usize) -> SearchResult {
    let started = Instant::now();
    let include_dirs = !filter.files_only && !filter.words.is_empty() && filter.exts.is_empty();
    let within = (!filter.within.is_empty()).then(|| resolve_within(index, &filter.within, places));
    let nodes = index.nodes();

    let mut ids: Vec<NodeId> = (1..nodes.len() as NodeId)
        .into_par_iter()
        .filter(|&id| {
            let n = &nodes[id as usize];
            !n.removed
                && (include_dirs || !n.is_dir)
                && matches(n, filter, now)
                && within.as_ref().is_none_or(|roots| has_ancestor_in(nodes, id, roots))
        })
        .collect();

    let total = ids.len() as u64;
    let order = |a: &NodeId, b: &NodeId| {
        let (na, nb) = (&nodes[*a as usize], &nodes[*b as usize]);
        nb.alloc.cmp(&na.alloc).then_with(|| na.name.cmp(&nb.name))
    };
    if ids.len() > limit {
        if limit > 0 {
            ids.select_nth_unstable_by(limit - 1, order);
        }
        ids.truncate(limit);
    }
    ids.sort_unstable_by(order);

    let results = ids
        .into_iter()
        .map(|id| {
            let n = &nodes[id as usize];
            Hit {
                id,
                name: n.name.to_string(),
                parent: if n.parent == NO_PARENT { String::new() } else { index.path_of(n.parent) },
                is_dir: n.is_dir,
                alloc: n.alloc,
                size: n.size,
                modified: n.modified,
                category: n.category,
            }
        })
        .collect();
    SearchResult { results, total, elapsed_ms: started.elapsed().as_millis() as u64 }
}

/// Every condition except `in:`, cheapest first.
fn matches(n: &Node, f: &Filter, now: i64) -> bool {
    f.kind.is_none_or(|k| n.category == k)
        && f.min_alloc.is_none_or(|min| n.alloc >= min)
        && f.max_alloc.is_none_or(|max| n.alloc <= max)
        && f.older_than.is_none_or(|age| n.modified < now - age)
        && f.newer_than.is_none_or(|age| n.modified > now - age)
        && (f.exts.is_empty() || extension(&n.name).is_some_and(|e| f.exts.iter().any(|x| x.eq_ignore_ascii_case(e))))
        && f.words.iter().all(|w| contains_ci(&n.name, w))
}

fn extension(name: &str) -> Option<&str> {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => Some(ext),
        _ => None,
    }
}

/// Case-insensitive substring test against an already-lowercase needle.
/// ASCII needles (the usual case) are compared without allocating.
fn contains_ci(hay: &str, needle: &str) -> bool {
    if needle.is_ascii() {
        let (h, n) = (hay.as_bytes(), needle.as_bytes());
        n.is_empty() || h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
    } else {
        hay.to_lowercase().contains(needle)
    }
}

/// Folders that `in:` values point at: a place key, a path, or folders with that name.
fn resolve_within(index: &Index, values: &[String], places: &[(String, String)]) -> HashSet<NodeId> {
    let mut roots = HashSet::new();
    for value in values {
        let place = places.iter().find(|(key, _)| key.eq_ignore_ascii_case(value));
        if let Some((_, path)) = place {
            roots.extend(index.find_path(path));
        } else if value.contains([':', '\\', '/']) {
            roots.extend(index.find_path(value));
        } else {
            let nodes = index.nodes();
            roots.extend((0..nodes.len() as NodeId).filter(|&id| {
                let n = &nodes[id as usize];
                n.is_dir && !n.removed && (n.name.eq_ignore_ascii_case(value) || n.name.to_lowercase() == value.to_lowercase())
            }));
        }
    }
    roots
}

fn has_ancestor_in(nodes: &[Node], id: NodeId, roots: &HashSet<NodeId>) -> bool {
    let mut cur = nodes[id as usize].parent;
    while cur != NO_PARENT {
        if roots.contains(&cur) {
            return true;
        }
        cur = nodes[cur as usize].parent;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categorize::Category;
    use crate::index::{DirTree, FileRec};

    const MB: u64 = 1 << 20;
    const GB: u64 = 1 << 30;
    const DAY: i64 = 86_400;
    const NOW: i64 = 1_800_000_000;

    fn file(name: &str, alloc: u64, age_days: i64) -> FileRec {
        FileRec { name: name.into(), size: alloc, alloc, modified: NOW - age_days * DAY }
    }

    fn dir(name: &str, files: Vec<FileRec>, dirs: Vec<DirTree>) -> DirTree {
        DirTree { name: name.into(), files, dirs, modified: NOW - DAY, ..Default::default() }
    }

    fn sample() -> Index {
        let downloads = dir(
            "Downloads",
            vec![file("Win11.iso", 5 * GB, 730), file("setup.exe", 30 * MB, 2), file("Report Final.pdf", 3 * MB, 40)],
            vec![],
        );
        let videos = dir("Videos", vec![file("holiday.mp4", 9 * GB, 800), file("clip.MKV", 200 * MB, 1)], vec![]);
        let modules = dir("node_modules", vec![file("big.js", 600 * MB, 10)], vec![]);
        let code = dir("code", vec![file("main.rs", 12 * 1024, 3)], vec![modules]);
        let sam = dir("sam", vec![], vec![downloads, videos, code]);
        let users = dir("Users", vec![], vec![sam]);
        let windows = dir("Windows", vec![file("pagefile.sys", 8 * GB, 5)], vec![]);
        Index::build(dir(r"C:\", vec![], vec![users, windows]))
    }

    fn places() -> Vec<(String, String)> {
        vec![
            ("downloads".into(), r"C:\Users\sam\Downloads".into()),
            ("desktop".into(), r"C:\Users\sam\Desktop".into()),
        ]
    }

    fn names(index: &Index, q: &str) -> Vec<String> {
        let filter = parse(q).unwrap();
        run(index, &filter, &places(), NOW, 100).results.into_iter().map(|h| h.name).collect()
    }

    #[test]
    fn name_words_must_all_match_case_insensitively() {
        let index = sample();
        assert_eq!(names(&index, "report"), ["Report Final.pdf"]);
        assert_eq!(names(&index, "FINAL rep"), ["Report Final.pdf"]);
        assert_eq!(names(&index, "final nope"), Vec::<String>::new());
    }

    #[test]
    fn empty_query_lists_files_largest_first() {
        let index = sample();
        let all = names(&index, "");
        assert_eq!(all[..3], ["holiday.mp4", "pagefile.sys", "Win11.iso"]);
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn ext_values_are_ored() {
        let index = sample();
        assert_eq!(names(&index, "ext:mkv,mp4"), ["holiday.mp4", "clip.MKV"]);
        assert_eq!(names(&index, "ext:iso ext:pdf"), ["Win11.iso", "Report Final.pdf"]);
    }

    #[test]
    fn size_bounds() {
        let index = sample();
        assert_eq!(names(&index, "size:>5gb"), ["holiday.mp4", "pagefile.sys"]);
        assert_eq!(names(&index, "size:>=5gb size:<9gb"), ["pagefile.sys", "Win11.iso"]);
        assert_eq!(names(&index, "size:<1mb"), ["main.rs"]);
    }

    #[test]
    fn age_bounds() {
        let index = sample();
        assert_eq!(names(&index, "age:>1y"), ["holiday.mp4", "Win11.iso"]);
        assert_eq!(names(&index, "age:<3d"), ["clip.MKV", "setup.exe"]);
    }

    #[test]
    fn kind_filter() {
        let index = sample();
        assert_eq!(names(&index, "kind:video"), ["holiday.mp4", "clip.MKV"]);
    }

    #[test]
    fn in_place_path_or_folder_name() {
        let index = sample();
        assert_eq!(names(&index, "in:downloads"), ["Win11.iso", "setup.exe", "Report Final.pdf"]);
        assert_eq!(names(&index, r"in:C:\Users\sam\Videos"), ["holiday.mp4", "clip.MKV"]);
        assert_eq!(names(&index, "in:node_modules"), ["big.js"]);
        assert_eq!(names(&index, "in:code size:<1mb"), ["main.rs"]);
        assert_eq!(names(&index, "in:downloads in:videos ext:mkv,iso"), ["Win11.iso", "clip.MKV"]);
        // A place that isn't part of this scan matches nothing.
        assert_eq!(names(&index, "in:desktop"), Vec::<String>::new());
        assert_eq!(names(&index, "in:nowhere"), Vec::<String>::new());
    }

    #[test]
    fn folders_match_only_with_name_words_and_no_ext() {
        let index = sample();
        assert_eq!(names(&index, "node_modules"), ["node_modules"]);
        assert_eq!(names(&index, "node_modules size:>500mb"), ["node_modules"]);
        assert_eq!(names(&index, "node_modules size:>1gb"), Vec::<String>::new());
        assert_eq!(
            names(&index, "o"),
            ["Videos", "holiday.mp4", "Windows", "Downloads", "Win11.iso", "code", "node_modules", "Report Final.pdf"]
        );
        assert_eq!(names(&index, "ext:js modules"), Vec::<String>::new());
    }

    #[test]
    fn hits_carry_parent_path_and_details() {
        let index = sample();
        let r = run(&index, &parse("holiday").unwrap(), &places(), NOW, 10);
        let hit = &r.results[0];
        assert_eq!(hit.parent, r"C:\Users\sam\Videos");
        assert_eq!((hit.alloc, hit.is_dir, hit.category), (9 * GB, false, Category::Video));
        assert_eq!(hit.modified, NOW - 800 * DAY);
        assert_eq!(index.path_of(hit.id), r"C:\Users\sam\Videos\holiday.mp4");
    }

    #[test]
    fn limit_caps_results_but_total_counts_all() {
        let index = sample();
        let r = run(&index, &Filter::default(), &places(), NOW, 2);
        assert_eq!(r.results.len(), 2);
        assert_eq!(r.total, 8);
        assert_eq!(r.results[0].name, "holiday.mp4");
    }

    #[test]
    fn removed_nodes_are_skipped() {
        let mut index = sample();
        index.remove(index.find_path(r"C:\Users\sam\Videos").unwrap());
        assert_eq!(names(&index, "ext:mp4,mkv"), Vec::<String>::new());
        assert_eq!(names(&index, "videos"), Vec::<String>::new());
    }

    #[test]
    fn biggest_filter_from_chips() {
        let index = sample();
        let biggest = |f: BiggestFilter| -> Vec<String> {
            run(&index, &biggest_filter(&f), &places(), NOW, 200).results.into_iter().map(|h| h.name).collect()
        };
        assert_eq!(biggest(BiggestFilter::default()).len(), 8);
        assert_eq!(
            biggest(BiggestFilter { min_size: Some(GB), ..Default::default() }),
            ["holiday.mp4", "pagefile.sys", "Win11.iso"]
        );
        assert_eq!(
            biggest(BiggestFilter { older_than_days: Some(365), ..Default::default() }),
            ["holiday.mp4", "Win11.iso"]
        );
        assert_eq!(
            biggest(BiggestFilter { category: Some(Category::Video), min_size: Some(100 * MB), ..Default::default() }),
            ["holiday.mp4", "clip.MKV"]
        );
    }
}
