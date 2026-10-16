import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { mockApi } from "./mockApi";
import type {
  AppsList,
  BiggestFilter,
  CleanItem,
  CleanupStatus,
  CompactStatus,
  DeleteStatus,
  Drive,
  IndexUpdated,
  NodeView,
  Opened,
  Place,
  Project,
  ScanDone,
  ScanProgress,
  SearchResult,
  FolderSize,
  HistoryDiff,
  HistoryList,
  RecycleReport,
  SystemItem,
  ToolId,
  ToolStatus,
  ToolView,
  UninstallStatus,
  VDisk,
} from "./types";

export type Unlisten = () => void;

export interface Api {
  listDrives(): Promise<Drive[]>;
  listPlaces(): Promise<Place[]>;
  /** Reopens the last scanned root from disk; null if nothing was saved. */
  restore(): Promise<Opened | null>;
  /** Loads `root` from its snapshot (refreshing in the background) or starts a scan. */
  openRoot(root: string): Promise<Opened>;
  scan(root: string): Promise<void>;
  cancelScan(): Promise<void>;
  getNode(id: number): Promise<NodeView>;
  nodeByPath(path: string): Promise<number | null>;
  /** The node at `path`, or its deepest ancestor still in the index. */
  nearestNode(path: string): Promise<number>;
  /** Rejects with the parser's message for a bad query. */
  search(query: string): Promise<SearchResult>;
  biggest(filter: BiggestFilter): Promise<SearchResult>;
  revealInExplorer(path: string): Promise<void>;
  openPath(path: string): Promise<void>;
  /** System Data rows; starts measuring when nothing is cached yet or `refresh` is set. */
  systemData(refresh: boolean): Promise<SystemItem[]>;
  isAdmin(): Promise<boolean>;
  /** Restarts Disklens elevated (one UAC prompt); rejects if Windows refused. */
  relaunchAsAdmin(): Promise<void>;
  /** Clean up rows, from System Data's latest measurements. */
  cleanupPlan(): Promise<CleanItem[]>;
  /** Moves the contents of these items to the Recycle Bin, in the background. */
  cleanupStart(ids: string[]): Promise<void>;
  cleanupStatus(): Promise<CleanupStatus>;
  openRecycleBin(): Promise<void>;
  /** Installed apps; discovers and measures in the background the first time or on `refresh`. */
  appsList(refresh: boolean): Promise<AppsList>;
  appUninstall(id: string): Promise<void>;
  appUninstallStatus(id: string): Promise<UninstallStatus | null>;
  appStopWaiting(id: string): Promise<void>;
  /** What an uninstalled program left behind. */
  appLeftovers(id: string): Promise<FolderSize[]>;
  /** Moves leftovers Disklens offered to the Recycle Bin. */
  recycleLeftovers(paths: string[]): Promise<RecycleReport>;
  /** Windows tools that apply to this machine right now. */
  toolsList(): Promise<ToolView[]>;
  /** Runs a tool: elevated commands in the background, dialogs right away. */
  toolRun(id: ToolId): Promise<void>;
  toolStatus(id: ToolId): Promise<ToolStatus | null>;
  /** WSL and Docker virtual disks. */
  vdisks(): Promise<VDisk[]>;
  /** Compacts these disks in the background (shuts WSL down, one UAC prompt). */
  vdiskCompact(paths: string[]): Promise<void>;
  vdiskStatus(): Promise<CompactStatus | null>;
  /** Earlier scans of the current root. */
  historyList(): Promise<HistoryList>;
  /** What changed between an earlier scan and the live index. */
  historyDiff(scannedAt: number): Promise<HistoryDiff>;
  /** Build folders across projects in the current scan; null when nothing is scanned. */
  projectsList(): Promise<Project[] | null>;
  /** Deletes build folders permanently, in the background. */
  projectsDelete(paths: string[]): Promise<void>;
  projectsStatus(): Promise<DeleteStatus>;
  onScanProgress(cb: (p: ScanProgress) => void): Promise<Unlisten>;
  onScanDone(cb: (d: ScanDone) => void): Promise<Unlisten>;
  onIndexUpdated(cb: (u: IndexUpdated) => void): Promise<Unlisten>;
}

const tauriApi: Api = {
  listDrives: () => invoke("list_drives"),
  listPlaces: () => invoke("list_places"),
  restore: () => invoke("restore"),
  openRoot: (root) => invoke("open_root", { root }),
  scan: (root) => invoke("scan", { root }),
  cancelScan: () => invoke("cancel_scan"),
  getNode: (id) => invoke("get_node", { id }),
  nodeByPath: (path) => invoke("node_by_path", { path }),
  nearestNode: (path) => invoke("nearest_node", { path }),
  search: (query) => invoke("search", { query }),
  biggest: (filter) => invoke("biggest", { filter }),
  revealInExplorer: (path) => invoke("reveal_in_explorer", { path }),
  openPath: (path) => invoke("open_path", { path }),
  systemData: (refresh) => invoke("system_data", { refresh }),
  isAdmin: () => invoke("is_admin"),
  relaunchAsAdmin: () => invoke("relaunch_as_admin"),
  cleanupPlan: () => invoke("cleanup_plan"),
  cleanupStart: (ids) => invoke("cleanup_start", { ids }),
  cleanupStatus: () => invoke("cleanup_status"),
  openRecycleBin: () => invoke("open_recycle_bin"),
  appsList: (refresh) => invoke("apps_list", { refresh }),
  appUninstall: (id) => invoke("app_uninstall", { id }),
  appUninstallStatus: (id) => invoke("app_uninstall_status", { id }),
  appStopWaiting: (id) => invoke("app_stop_waiting", { id }),
  appLeftovers: (id) => invoke("app_leftovers", { id }),
  recycleLeftovers: (paths) => invoke("recycle_leftovers", { paths }),
  toolsList: () => invoke("tools_list"),
  toolRun: (id) => invoke("tool_run", { id }),
  toolStatus: (id) => invoke("tool_status", { id }),
  vdisks: () => invoke("vdisks"),
  vdiskCompact: (paths) => invoke("vdisk_compact", { paths }),
  vdiskStatus: () => invoke("vdisk_status"),
  historyList: () => invoke("history_list"),
  historyDiff: (scannedAt) => invoke("history_diff", { scannedAt }),
  projectsList: () => invoke("projects_list"),
  projectsDelete: (paths) => invoke("projects_delete", { paths }),
  projectsStatus: () => invoke("projects_status"),
  onScanProgress: (cb) => listen<ScanProgress>("scan-progress", (e) => cb(e.payload)),
  onScanDone: (cb) => listen<ScanDone>("scan-done", (e) => cb(e.payload)),
  onIndexUpdated: (cb) => listen<IndexUpdated>("index-updated", (e) => cb(e.payload)),
};

/** Outside the Tauri window (plain `npm run dev` in a browser) the UI runs on fake data. */
export const api: Api = "__TAURI_INTERNALS__" in window ? tauriApi : mockApi;
