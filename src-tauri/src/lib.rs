pub mod activity;
pub mod apps;
pub mod categorize;
pub mod cleanup;
mod commands;
mod drives;
pub mod elevation;
pub mod history;
pub mod index;
pub mod projects;
pub mod remove;
pub mod scanner;
pub mod search;
pub mod session;
pub mod snapshot;
pub mod system_data;
pub mod tools;
pub mod vdisk;
pub mod watcher;
mod win;

use scanner::WalkScanner;
use session::{Event, Session};
use snapshot::Store;
use apps::Apps;
use cleanup::{recycle::recycle, rules::RULES, running::process_names, Cleanup};
use system_data::{catalog::CATALOG, expand::env_lookup, SystemData};
use std::sync::Arc;
use tauri::{Emitter, Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(SystemData::new(CATALOG, Arc::new(env_lookup), elevation::is_elevated()))
        .manage(Cleanup::new(CATALOG, RULES, Arc::new(env_lookup), Arc::new(process_names), recycle))
        .manage(Apps::new(Arc::new(env_lookup), recycle))
        .manage(tools::Tools::new(Arc::new(env_lookup)))
        .manage(vdisk::VDisks::new(Arc::new(env_lookup)))
        .manage(projects::Projects::new(Arc::new(env_lookup)))
        .manage(activity::Activity::new())
        .setup(|app| {
            let handle = app.handle().clone();
            let emit = Arc::new(move |event: Event| {
                let _ = match event {
                    Event::Progress(p) => handle.emit("scan-progress", p),
                    Event::ScanDone(d) => handle.emit("scan-done", d),
                    Event::IndexUpdated(u) => handle.emit("index-updated", u),
                };
            });
            let store = Store::default_dir().map(Store::new);
            app.manage(Session::new(store, Arc::new(WalkScanner), emit));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_drives,
            commands::list_places,
            commands::restore,
            commands::open_root,
            commands::scan,
            commands::cancel_scan,
            commands::get_node,
            commands::node_by_path,
            commands::nearest_node,
            commands::search,
            commands::biggest,
            commands::reveal_in_explorer,
            commands::open_path,
            system_data::system_data,
            elevation::is_admin,
            elevation::relaunch_as_admin,
            cleanup::cleanup_plan,
            cleanup::cleanup_start,
            cleanup::cleanup_status,
            cleanup::open_recycle_bin,
            apps::apps_list,
            apps::app_uninstall,
            apps::app_uninstall_status,
            apps::app_stop_waiting,
            apps::app_leftovers,
            apps::recycle_leftovers,
            tools::tools_list,
            tools::tool_run,
            tools::tool_status,
            vdisk::vdisks,
            vdisk::vdisk_compact,
            vdisk::vdisk_status,
            history::history_list,
            history::history_diff,
            projects::projects_list,
            projects::projects_delete,
            projects::projects_status,
            activity::activity_snapshot,
            activity::activity_end,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Disklens")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<Session>().save_if_dirty();
            }
        });
}
