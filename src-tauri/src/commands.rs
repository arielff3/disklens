//! Tauri commands: the only surface the UI talks to. The state behind them is a
//! [`Session`]; this module only translates.

use crate::drives::{self, Drive, Place};
use crate::index::{NodeId, NodeView};
use crate::search::{self, BiggestFilter, SearchResult};
use crate::session::{unix_now, Opened, Session};
use tauri::State;

/// Children sent per folder; the rest is summarized as one "smaller items" row.
const MAX_CHILDREN: usize = 500;
const MAX_SEARCH_RESULTS: usize = 500;
const MAX_BIGGEST: usize = 200;

#[tauri::command]
pub fn list_drives() -> Vec<Drive> {
    drives::list_drives()
}

#[tauri::command]
pub fn list_places() -> Vec<Place> {
    drives::list_places()
}

/// Reopens the last scanned root from its snapshot, if any.
#[tauri::command]
pub fn restore(session: State<'_, Session>) -> Result<Option<Opened>, String> {
    session.restore()
}

/// Shows `root` from its snapshot (refreshing in the background) or scans it.
#[tauri::command]
pub fn open_root(session: State<'_, Session>, root: String) -> Result<Opened, String> {
    session.open_root(&root)
}

/// Starts a foreground scan of `root`, cancelling any scan in progress.
/// Progress arrives as `scan-progress` events, the result as `scan-done`.
#[tauri::command]
pub fn scan(session: State<'_, Session>, root: String) -> Result<(), String> {
    session.scan(&root)
}

#[tauri::command]
pub fn cancel_scan(session: State<'_, Session>) {
    session.cancel_scan();
}

#[tauri::command]
pub fn get_node(session: State<'_, Session>, id: NodeId) -> Result<NodeView, String> {
    session
        .with_index(|i| i.view(id, MAX_CHILDREN))
        .ok_or_else(not_scanned)?
        .ok_or_else(|| format!("node {id} not found"))
}

#[tauri::command]
pub fn node_by_path(session: State<'_, Session>, path: String) -> Result<Option<NodeId>, String> {
    session.with_index(|i| i.find_path(&path)).ok_or_else(not_scanned)
}

/// The node at `path` or its deepest ancestor still in the index; the root if
/// the path is outside it.
#[tauri::command]
pub fn nearest_node(session: State<'_, Session>, path: String) -> Result<NodeId, String> {
    session.with_index(|i| i.nearest(&path).unwrap_or(i.root())).ok_or_else(not_scanned)
}

#[tauri::command]
pub fn search(session: State<'_, Session>, query: String) -> Result<SearchResult, String> {
    let filter = search::parse(&query).map_err(|e| e.to_string())?;
    let places = place_keys();
    session
        .with_index(|i| search::run(i, &filter, &places, unix_now(), MAX_SEARCH_RESULTS))
        .ok_or_else(not_scanned)
}

#[tauri::command]
pub fn biggest(session: State<'_, Session>, filter: BiggestFilter) -> Result<SearchResult, String> {
    let filter = search::biggest_filter(&filter);
    session.with_index(|i| search::run(i, &filter, &[], unix_now(), MAX_BIGGEST)).ok_or_else(not_scanned)
}

#[tauri::command]
pub fn reveal_in_explorer(path: String) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| e.to_string())
}

fn not_scanned() -> String {
    "Nothing scanned yet".to_string()
}

/// Place keys as `in:` accepts them (`programfiles`), with their paths.
fn place_keys() -> Vec<(String, String)> {
    drives::list_places().into_iter().map(|p| (p.key.to_ascii_lowercase(), p.path)).collect()
}
