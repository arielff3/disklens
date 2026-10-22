export type Category =
  | "video"
  | "images"
  | "audio"
  | "archives"
  | "code"
  | "documents"
  | "apps"
  | "dataCaches"
  | "other";

export type PlaceKey =
  | "home"
  | "downloads"
  | "documents"
  | "desktop"
  | "pictures"
  | "videos"
  | "appdata"
  | "programFiles";

export interface Drive {
  root: string;
  label: string;
  fs: string;
  total: number;
  free: number;
  removable: boolean;
}

export interface Place {
  key: PlaceKey;
  path: string;
}

export interface CategorySize {
  category: Category;
  alloc: number;
}

export interface Crumb {
  id: number;
  name: string;
}

export interface ChildView {
  id: number;
  name: string;
  isDir: boolean;
  alloc: number;
  size: number;
  files: number;
  category: Category;
  unreadable: boolean;
}

export interface NodeView {
  id: number;
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  alloc: number;
  files: number;
  modified: number;
  unreadable: boolean;
  categories: CategorySize[];
  crumbs: Crumb[];
  children: ChildView[];
  rest: { count: number; alloc: number } | null;
}

export interface ScanProgress {
  files: number;
  bytes: number;
  current: string;
}

export interface ScanDone {
  root: string;
  ok: boolean;
  error: string | null;
  cancelled: boolean;
  files: number;
  unreadable: number;
  elapsedMs: number;
}

export interface Opened {
  root: string;
  /** True when a saved index was loaded; false means a foreground scan started. */
  loaded: boolean;
  scannedAt: number;
  files: number;
  unreadable: number;
}

export interface IndexUpdated {
  root: string;
  files: number;
  unreadable: number;
  /** A background scan is still running. */
  refreshing: boolean;
  scannedAt: number;
}

export interface Hit {
  id: number;
  name: string;
  /** Path of the containing folder. */
  parent: string;
  isDir: boolean;
  alloc: number;
  size: number;
  modified: number;
  category: Category;
}

export interface SearchResult {
  results: Hit[];
  total: number;
  elapsedMs: number;
}

export interface BiggestFilter {
  minSize: number | null;
  olderThanDays: number | null;
  category: Category | null;
}

export type SystemGroup = "windows" | "developer" | "appCaches" | "tempLogs";
export type Verdict = "safeToClear" | "lookFirst" | "managedByWindows";
export type ItemState = "pending" | "ok" | "partial" | "needsAdmin" | "missing";
export type SystemNote = "sharedWithWindows" | "measuringWithDism" | "measuredByDism";

export interface SystemItem {
  id: string;
  group: SystemGroup;
  verdict: Verdict;
  state: ItemState;
  alloc: number;
  files: number;
  path: string | null;
  isDir: boolean;
  note: SystemNote | null;
}

export interface CleanItem {
  id: string;
  group: SystemGroup;
  state: ItemState;
  alloc: number;
  files: number;
  defaultOn: boolean;
  /** The app to close first, if it is running. */
  appOpen: string | null;
  /** Folders (or files) whose contents would be moved. */
  roots: string[];
}

export interface CleanupStatus {
  running: boolean;
  finished: boolean;
  total: number;
  done: number;
  current: string | null;
  /** Bytes moved to the Recycle Bin. */
  moved: number;
  inUse: number;
  inUseExamples: string[];
  /** Ids skipped because their app was open. */
  appsOpen: string[];
  refused: string[];
}

export type AppKind = "program" | "store";

export interface FolderSize {
  path: string;
  alloc: number;
}

export interface AppView {
  id: string;
  kind: AppKind;
  name: string;
  publisher: string | null;
  version: string | null;
  installLocation: string | null;
  installSize: number;
  /** The install size is the installer's estimate, not a measurement. */
  estimated: boolean;
  dataFolders: FolderSize[];
  dataSize: number;
  measured: boolean;
}

export interface AppsList {
  loading: boolean;
  apps: AppView[];
}

export type UninstallPhase = "running" | "done" | "failed" | "stopped";

export interface UninstallStatus {
  phase: UninstallPhase;
  message: string | null;
}

export interface RecycleReport {
  moved: number;
  left: string[];
  refused: string[];
}

export type ToolId = "hibernation" | "componentCleanup" | "compactOs" | "systemProtection" | "virtualMemory";

export interface ToolView {
  id: ToolId;
  available: boolean;
  /** For CompactOS: whether Windows files are already compressed. */
  on: boolean | null;
  /** Runs a command as administrator, rather than opening a Windows window. */
  elevated: boolean;
}

export interface ToolStatus {
  phase: "running" | "done" | "failed";
  message: string | null;
}

export interface VDisk {
  path: string;
  kind: "wsl" | "docker";
  /** The WSL distribution using it, when known. */
  distro: string | null;
  alloc: number;
}

export interface CompactStatus {
  phase: "trimming" | "shuttingDown" | "compacting" | "done" | "failed";
  message: string | null;
  results: { path: string; before: number; after: number }[];
}

export interface HistoryPoint {
  scannedAt: number;
  total: number;
}

export interface HistoryList {
  root: string | null;
  /** When the index being shown was scanned. */
  currentAt: number | null;
  currentTotal: number;
  /** Earlier scans to compare with, newest first. */
  points: HistoryPoint[];
}

export interface FolderChange {
  /** Absolute path. */
  path: string;
  before: number;
  after: number;
}

export interface HistoryDiff {
  beforeAt: number;
  beforeTotal: number;
  afterTotal: number;
  changes: FolderChange[];
}

export type JunkKind = "node" | "rust" | "gradle" | "dotnet" | "python" | "next";

export interface JunkFolder {
  path: string;
  kind: JunkKind;
  alloc: number;
  files: number;
}

export interface Project {
  path: string;
  name: string;
  /** Newest file outside build and hidden folders (Unix seconds). */
  lastActive: number;
  folders: JunkFolder[];
  total: number;
}

export interface DeleteStatus {
  running: boolean;
  finished: boolean;
  total: number;
  done: number;
  current: string | null;
  freed: number;
  failed: string[];
  refused: string[];
}

/** One process in Activity. CPU is percent of the whole machine; memory is the private working set. */
export interface ActivityProcess {
  pid: number;
  parent: number | null;
  name: string;
  exe: string | null;
  cpu: number;
  ram: number;
  /** null when the GPU counters aren't available. */
  gpu: number | null;
  gpuMem: number | null;
}

/** Processes grouped by executable. `name` is empty for the Windows processes group. */
export interface ActivityApp {
  key: string;
  name: string;
  exe: string | null;
  processes: ActivityProcess[];
  cpu: number;
  ram: number;
  gpu: number | null;
  gpuMem: number | null;
}

export interface SystemNow {
  cpu: number;
  cores: number;
  ramUsed: number;
  ramTotal: number;
  gpu: number | null;
  gpuMemUsed: number | null;
  gpuMemTotal: number | null;
}

export interface SystemPoint {
  cpu: number;
  ramUsed: number;
  gpu: number | null;
}

export interface ActivitySnapshot {
  takenAt: number;
  system: SystemNow;
  /** The last minute, oldest first, one point a second. */
  history: SystemPoint[];
  apps: ActivityApp[];
}

export type EndOutcome = "ended" | "gone" | "denied" | "refused" | "failed";

export interface EndResult {
  pid: number;
  outcome: EndOutcome;
}
