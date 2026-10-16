import type { Api } from "./api";
import { isWithin, joinPath, samePath } from "./paths";
import type {
  AppView,
  Category,
  CleanItem,
  CleanupStatus,
  CompactStatus,
  DeleteStatus,
  HistoryDiff,
  Hit,
  IndexUpdated,
  NodeView,
  Project,
  ScanDone,
  ScanProgress,
  SearchResult,
  SystemItem,
  ToolId,
  ToolView,
  VDisk,
} from "./types";

const MB = 1024 ** 2;
const GB = 1024 ** 3;
const DAY = 86_400;
const NOW = Math.floor(Date.now() / 1000);

type Spec = readonly [name: string, size: number, category: Category] | readonly [name: string, children: Spec[]];

const TREE: Spec[] = [
  ["Users", [["you", [
    ["AppData", [
      ["Local", [["Temp", [["setup-cache.bin", 3.2 * GB, "dataCaches"]]], ["npm-cache.tar", 1.4 * GB, "archives"], ["chrome-cache.dat", 2.6 * GB, "dataCaches"]]],
      ["Roaming", [["Code.log", 0.4 * GB, "dataCaches"]]],
    ]],
    ["Videos", [["holiday.mp4", 9.2 * GB, "video"], ["talk.mkv", 5.1 * GB, "video"]]],
    ["Pictures", [["Camera Roll", [["IMG_2041.heic", 4.3 * GB, "images"]]], ["wallpaper.png", 12 * MB, "images"]]],
    ["Downloads", [["Win11_24H2.iso", 5.4 * GB, "archives"], ["node-v22.msi", 31 * MB, "apps"], ["report.pdf", 3 * MB, "documents"]]],
    ["Documents", [["thesis.docx", 48 * MB, "documents"], ["budget.xlsx", 2 * MB, "documents"]]],
    ["code", [["disklens", [["deps.rlib", 2.1 * GB, "code"], ["main.rs", 12_000, "code"]]]]],
  ]]]],
  ["Windows", [["WinSxS", [["components.dat", 7.8 * GB, "dataCaches"]]], ["System32", [["drivers.sys", 3.9 * GB, "apps"]]]]],
  ["Program Files", [["Steam", [["game.pak", 18.4 * GB, "dataCaches"]]], ["Docker", [["docker.vhdx", 6.2 * GB, "archives"]]]]],
  ["pagefile.sys", 8 * GB, "apps"],
  ["hiberfil.sys", 6.4 * GB, "apps"],
];

interface MockNode {
  name: string;
  parent: number | null;
  isDir: boolean;
  alloc: number;
  files: number;
  modified: number;
  category: Category;
  cats: Partial<Record<Category, number>>;
  children: number[];
}

function build(): MockNode[] {
  const nodes: MockNode[] = [];
  const addDir = (name: string, parent: number | null, specs: Spec[]): number => {
    const id = nodes.length;
    const node: MockNode = { name, parent, isDir: true, alloc: 0, files: 0, modified: NOW - DAY, category: "other", cats: {}, children: [] };
    nodes.push(node);
    for (const spec of specs) {
      let child: number;
      if (typeof spec[1] === "number") {
        const [fileName, size, category] = spec as readonly [string, number, Category];
        child = nodes.length;
        const alloc = Math.round(size);
        const modified = NOW - ((child * 97) % 900) * DAY;
        nodes.push({ name: fileName, parent: id, isDir: false, alloc, files: 1, modified, category, cats: { [category]: alloc }, children: [] });
      } else {
        child = addDir(spec[0], id, spec[1] as Spec[]);
      }
      const c = nodes[child];
      node.alloc += c.alloc;
      node.files += c.files;
      node.children.push(child);
      for (const [k, v] of Object.entries(c.cats)) {
        node.cats[k as Category] = (node.cats[k as Category] ?? 0) + (v ?? 0);
      }
    }
    node.children.sort((a, b) => nodes[b].alloc - nodes[a].alloc);
    const top = Object.entries(node.cats).sort((a, b) => (b[1] ?? 0) - (a[1] ?? 0))[0];
    node.category = (top?.[0] as Category | undefined) ?? "other";
    return id;
  };
  addDir("C:\\", null, TREE);
  return nodes;
}

const nodes = build();

function pathOf(id: number): string {
  const n = nodes[id];
  return n.parent === null ? n.name : joinPath(pathOf(n.parent), n.name);
}

function view(id: number): NodeView {
  const n = nodes[id];
  const crumbs: { id: number; name: string }[] = [];
  for (let c: number | null = id; c !== null; c = nodes[c].parent) crumbs.unshift({ id: c, name: nodes[c].name });
  const categories = Object.entries(n.cats)
    .map(([category, alloc]) => ({ category: category as Category, alloc: alloc ?? 0 }))
    .filter((c) => c.alloc > 0)
    .sort((a, b) => b.alloc - a.alloc);
  const children = n.children.map((c) => {
    const k = nodes[c];
    return { id: c, name: k.name, isDir: k.isDir, alloc: k.alloc, size: k.alloc, files: k.files, category: k.category, unreadable: false };
  });
  return { id, name: n.name, path: pathOf(id), isDir: n.isDir, size: n.alloc, alloc: n.alloc, files: n.files, modified: 0, unreadable: false, categories, crumbs, children, rest: null };
}

function hit(id: number): Hit {
  const n = nodes[id];
  return { id, name: n.name, parent: n.parent === null ? "" : pathOf(n.parent), isDir: n.isDir, alloc: n.alloc, size: n.alloc, modified: n.modified, category: n.category };
}

function top(ids: number[], limit: number, started: number): SearchResult {
  ids.sort((a, b) => nodes[b].alloc - nodes[a].alloc);
  return { results: ids.slice(0, limit).map(hit), total: ids.length, elapsedMs: Math.round(performance.now() - started) };
}

const UNIT: Record<string, number> = { "": 1, b: 1, kb: 1024, mb: MB, gb: GB, tb: 1024 * GB };

/** A rough stand-in for the Rust parser: words, ext:, size:>/<, kind:. */
function mockSearch(query: string): SearchResult {
  const started = performance.now();
  const words: string[] = [];
  const exts: string[] = [];
  let min = 0;
  let max = Infinity;
  let kind: string | null = null;
  for (const term of query.split(/\s+/).filter(Boolean)) {
    const m = /^([a-z]{2,}):(.*)$/i.exec(term);
    if (!m) {
      words.push(term.toLowerCase());
      continue;
    }
    const [, key, value] = m;
    const size = /^([<>]=?)?([\d.]+)([a-z]*)$/i.exec(value);
    if (key === "ext" && value) exts.push(...value.toLowerCase().split(",").map((e) => e.replace(/^\./, "")));
    else if (key === "kind" && value) kind = value.toLowerCase();
    else if (key === "size" && size && UNIT[size[3].toLowerCase()] !== undefined) {
      const bytes = Number(size[2]) * UNIT[size[3].toLowerCase()];
      if (size[1]?.startsWith("<")) max = bytes;
      else min = bytes;
    } else throw `${term}: unknown filter; use ext:, size:, age:, in: or kind:`;
  }
  const dirsToo = words.length > 0 && exts.length === 0;
  const ids = nodes
    .map((_, id) => id)
    .filter((id) => {
      const n = nodes[id];
      const ext = n.name.includes(".") ? n.name.split(".").pop()!.toLowerCase() : "";
      return (
        id !== 0 &&
        (dirsToo || !n.isDir) &&
        n.alloc >= min &&
        n.alloc <= max &&
        (kind === null || n.category.toLowerCase().startsWith(kind)) &&
        (exts.length === 0 || exts.includes(ext)) &&
        words.every((w) => n.name.toLowerCase().includes(w))
      );
    });
  return top(ids, 500, started);
}

const progressSubs = new Set<(p: ScanProgress) => void>();
const doneSubs = new Set<(d: ScanDone) => void>();
let timer: number | undefined;
const indexSubs = new Set<(u: IndexUpdated) => void>();

const SYSTEM_ITEMS: SystemItem[] = [
  { id: "pagefile", group: "windows", verdict: "managedByWindows", state: "ok", alloc: 8 * GB, files: 1, path: "C:\\pagefile.sys", isDir: false, note: null },
  { id: "hiberfil", group: "windows", verdict: "lookFirst", state: "ok", alloc: 6.4 * GB, files: 1, path: "C:\\hiberfil.sys", isDir: false, note: null },
  { id: "componentStore", group: "windows", verdict: "managedByWindows", state: "partial", alloc: 9.8 * GB, files: 81_000, path: "C:\\Windows\\WinSxS", isDir: true, note: "sharedWithWindows" },
  { id: "updateDownloads", group: "windows", verdict: "safeToClear", state: "ok", alloc: 1.2 * GB, files: 340, path: "C:\\Windows\\SoftwareDistribution\\Download", isDir: true, note: null },
  { id: "restorePoints", group: "windows", verdict: "lookFirst", state: "needsAdmin", alloc: 0, files: 0, path: null, isDir: false, note: null },
  { id: "recycleBin", group: "windows", verdict: "lookFirst", state: "ok", alloc: 2.1 * GB, files: 52, path: null, isDir: false, note: null },
  { id: "npmCache", group: "developer", verdict: "safeToClear", state: "ok", alloc: 3.4 * GB, files: 120_000, path: "C:\\Users\\you\\AppData\\Local\\npm-cache", isDir: true, note: null },
  { id: "cargoRegistry", group: "developer", verdict: "safeToClear", state: "ok", alloc: 2.2 * GB, files: 61_000, path: "C:\\Users\\you\\.cargo\\registry", isDir: true, note: null },
  { id: "wslDisks", group: "developer", verdict: "lookFirst", state: "ok", alloc: 21.5 * GB, files: 1, path: "C:\\Users\\you\\AppData\\Local\\wsl\\ext4.vhdx", isDir: false, note: null },
  { id: "chromeCache", group: "appCaches", verdict: "safeToClear", state: "ok", alloc: 860 * MB, files: 4_100, path: "C:\\Users\\you\\AppData\\Local\\Google\\Chrome\\User Data", isDir: true, note: null },
  { id: "spotifyCache", group: "appCaches", verdict: "safeToClear", state: "ok", alloc: 1.4 * GB, files: 900, path: "C:\\Users\\you\\AppData\\Local\\Spotify\\Storage", isDir: true, note: null },
  { id: "userTemp", group: "tempLogs", verdict: "safeToClear", state: "ok", alloc: 2.7 * GB, files: 15_000, path: "C:\\Users\\you\\AppData\\Local\\Temp", isDir: true, note: null },
  { id: "windowsTemp", group: "tempLogs", verdict: "safeToClear", state: "needsAdmin", alloc: 0, files: 0, path: null, isDir: false, note: null },
];
let systemStarted = 0;

const CLEAN_ITEMS: CleanItem[] = [
  { id: "chromeCache", group: "appCaches", state: "ok", alloc: 2.6 * GB, files: 26_000, defaultOn: true, appOpen: null, roots: ["C:\\Users\\you\\AppData\\Local\\Google\\Chrome\\User Data\\Default\\Cache"] },
  { id: "spotifyCache", group: "appCaches", state: "ok", alloc: 1.4 * GB, files: 900, defaultOn: true, appOpen: "Spotify", roots: ["C:\\Users\\you\\AppData\\Local\\Spotify\\Storage"] },
  { id: "teamsCache", group: "appCaches", state: "ok", alloc: 610 * MB, files: 3_800, defaultOn: true, appOpen: null, roots: ["C:\\Users\\you\\AppData\\Local\\Packages\\MSTeams_8wekyb3d8bbwe\\LocalCache"] },
  { id: "userTemp", group: "tempLogs", state: "partial", alloc: 2.7 * GB, files: 15_000, defaultOn: true, appOpen: null, roots: ["C:\\Users\\you\\AppData\\Local\\Temp"] },
  { id: "crashDumps", group: "tempLogs", state: "ok", alloc: 620 * MB, files: 10, defaultOn: true, appOpen: null, roots: ["C:\\Users\\you\\AppData\\Local\\CrashDumps"] },
  { id: "windowsTemp", group: "tempLogs", state: "needsAdmin", alloc: 0, files: 0, defaultOn: true, appOpen: null, roots: [] },
  { id: "npmCache", group: "developer", state: "ok", alloc: 700 * MB, files: 24_000, defaultOn: false, appOpen: null, roots: ["C:\\Users\\you\\AppData\\Local\\npm-cache"] },
  { id: "gradleCache", group: "developer", state: "ok", alloc: 980 * MB, files: 13_000, defaultOn: false, appOpen: null, roots: ["C:\\Users\\you\\.gradle\\caches"] },
];
const APPS: AppView[] = [
  { id: "HKCU\\Discord", kind: "program", name: "Discord", publisher: "Discord Inc.", version: "1.0.9", installLocation: "C:\\Users\\you\\AppData\\Local\\Discord", installSize: 0.35 * GB, estimated: false, dataFolders: [{ path: "C:\\Users\\you\\AppData\\Roaming\\discord", alloc: 1.1 * GB }], dataSize: 1.1 * GB, measured: true },
  { id: "HKLM\\Chrome", kind: "program", name: "Google Chrome", publisher: "Google LLC", version: "141.0", installLocation: "C:\\Program Files\\Google\\Chrome\\Application", installSize: 0.5 * GB, estimated: false, dataFolders: [{ path: "C:\\Users\\you\\AppData\\Local\\Google\\Chrome", alloc: 13.2 * GB }], dataSize: 13.2 * GB, measured: true },
  { id: "HKLM\\Docker", kind: "program", name: "Docker Desktop", publisher: "Docker Inc.", version: "4.40", installLocation: "C:\\Program Files\\Docker\\Docker", installSize: 2.7 * GB, estimated: false, dataFolders: [{ path: "C:\\Users\\you\\AppData\\Local\\Docker", alloc: 33.1 * GB }], dataSize: 33.1 * GB, measured: true },
  { id: "HKLM\\Premiere", kind: "program", name: "Adobe Premiere Pro 2026", publisher: "Adobe Inc.", version: "26.0", installLocation: null, installSize: 10 * GB, estimated: true, dataFolders: [], dataSize: 0, measured: true },
  { id: "store:Spotify", kind: "store", name: "Spotify", publisher: "Spotify AB", version: "1.2.3.0", installLocation: "C:\\Program Files\\WindowsApps\\SpotifyAB.SpotifyMusic_1.2.3.0_x64", installSize: 0.3 * GB, estimated: false, dataFolders: [{ path: "C:\\Users\\you\\AppData\\Local\\Packages\\SpotifyAB.SpotifyMusic_zpdnekdrzrea0", alloc: 1.7 * GB }], dataSize: 1.7 * GB, measured: true },
  { id: "HKLM\\Ollama", kind: "program", name: "Ollama", publisher: "Ollama", version: "0.24.0", installLocation: "C:\\Users\\you\\AppData\\Local\\Programs\\Ollama", installSize: 6.5 * GB, estimated: false, dataFolders: [{ path: "C:\\Users\\you\\AppData\\Local\\Ollama", alloc: 1.5 * GB }], dataSize: 1.5 * GB, measured: true },
];
const uninstalls = new Map<string, number>();
const tools: ToolView[] = [
  { id: "hibernation", available: true, on: null, elevated: true },
  { id: "componentCleanup", available: true, on: null, elevated: true },
  { id: "compactOs", available: true, on: false, elevated: true },
  { id: "systemProtection", available: true, on: null, elevated: false },
  { id: "virtualMemory", available: true, on: null, elevated: false },
];
const toolRuns = new Map<ToolId, number>();
const DISKS: VDisk[] = [
  { path: "C:\\Users\\you\\AppData\\Local\\Packages\\KaliLinux\\LocalState\\ext4.vhdx", kind: "wsl", distro: "kali-linux", alloc: 57.6 * GB },
  { path: "C:\\Users\\you\\AppData\\Local\\Docker\\wsl\\disk\\docker_data.vhdx", kind: "docker", distro: null, alloc: 33.1 * GB },
  { path: "C:\\Users\\you\\AppData\\Local\\Packages\\Ubuntu\\LocalState\\ext4.vhdx", kind: "wsl", distro: "Ubuntu", alloc: 11.1 * GB },
];
let compactStarted = 0;
const HOUR = 3600;
const NOW_S = Math.floor(Date.now() / 1000);
const PROJECTS: Project[] = [
  { path: "C:\\Users\\you\\projects\\desktop-app\\src-tauri", name: "src-tauri", lastActive: NOW_S - 87 * 24 * HOUR, folders: [{ path: "C:\\Users\\you\\projects\\desktop-app\\src-tauri\\target", kind: "rust", alloc: 8.68 * GB, files: 1000 }], total: 8.68 * GB },
  { path: "C:\\Users\\you\\projects\\speech-demo", name: "speech-demo", lastActive: NOW_S - 17 * 24 * HOUR, folders: [{ path: "C:\\Users\\you\\projects\\speech-demo\\.venv", kind: "python", alloc: 5.55 * GB, files: 1000 }], total: 5.55 * GB },
  { path: "C:\\Users\\you\\projects\\web-dashboard", name: "web-dashboard", lastActive: NOW_S - 0 * 24 * HOUR, folders: [{ path: "C:\\Users\\you\\projects\\web-dashboard\\node_modules", kind: "node", alloc: 2.93 * GB, files: 1000 }, { path: "C:\\Users\\you\\projects\\web-dashboard\\.next", kind: "next", alloc: 1.62 * GB, files: 1000 }], total: 2.93 * GB + 1.62 * GB },
  { path: "C:\\Users\\you\\projects\\search-ui", name: "search-ui", lastActive: NOW_S - 428 * 24 * HOUR, folders: [{ path: "C:\\Users\\you\\projects\\search-ui\\node_modules", kind: "node", alloc: 1.35 * GB, files: 1000 }, { path: "C:\\Users\\you\\projects\\search-ui\\.next", kind: "next", alloc: 1.17 * GB, files: 1000 }], total: 1.35 * GB + 1.17 * GB },
  { path: "C:\\Users\\you\\projects\\algorithms-site", name: "algorithms-site", lastActive: NOW_S - 567 * 24 * HOUR, folders: [{ path: "C:\\Users\\you\\projects\\algorithms-site\\node_modules", kind: "node", alloc: 1.95 * GB, files: 1000 }, { path: "C:\\Users\\you\\projects\\algorithms-site\\.next", kind: "next", alloc: 0.19 * GB, files: 1000 }], total: 1.95 * GB + 0.19 * GB },
  { path: "C:\\Users\\you\\projects\\next.js", name: "next.js", lastActive: NOW_S - 524 * 24 * HOUR, folders: [{ path: "C:\\Users\\you\\projects\\next.js\\node_modules", kind: "node", alloc: 1.83 * GB, files: 1000 }], total: 1.83 * GB },
];
let deleted = new Set<string>();
let deleteStatus: DeleteStatus = { running: false, finished: false, total: 0, done: 0, current: null, freed: 0, failed: [], refused: [] };
const HISTORY_DIFF: HistoryDiff = {
  beforeAt: NOW_S - 6 * 24 * HOUR,
  beforeTotal: 72.2 * GB,
  afterTotal: 84.49 * GB,
  changes: [
    { path: "C:\\Users\\you\\AppData\\Local\\Docker", before: 24.1 * GB, after: 33.1 * GB },
    { path: "C:\\Users\\you\\Downloads", before: 3.3 * GB, after: 5.4 * GB },
    { path: "C:\\Users\\you\\AppData\\Local\\Temp", before: 3.5 * GB, after: 2.7 * GB },
    { path: "C:\\Windows\\SoftwareDistribution\\Download", before: 0.3 * GB, after: 1.2 * GB },
    { path: "C:\\Users\\you\\code\\disklens\\target", before: 0, after: 2.1 * GB },
  ],
};
let compactPaths: string[] = [];
let cleanStatus: CleanupStatus = { running: false, finished: false, total: 0, done: 0, current: null, moved: 0, inUse: 0, inUseExamples: [], appsOpen: [], refused: [] };

export const mockApi: Api = {
  listDrives: async () => [
    { root: "C:\\", label: "Windows", fs: "NTFS", total: 476 * GB, free: 132 * GB, removable: false },
    { root: "D:\\", label: "Data", fs: "NTFS", total: 931 * GB, free: 610 * GB, removable: false },
  ],
  listPlaces: async () => [
    { key: "home", path: "C:\\Users\\you" },
    { key: "downloads", path: "C:\\Users\\you\\Downloads" },
    { key: "documents", path: "C:\\Users\\you\\Documents" },
    { key: "pictures", path: "C:\\Users\\you\\Pictures" },
    { key: "videos", path: "C:\\Users\\you\\Videos" },
    { key: "appdata", path: "C:\\Users\\you\\AppData" },
    { key: "programFiles", path: "C:\\Program Files" },
  ],
  // `?demo` (browser only, used for README screenshots) opens the fake C: right away.
  restore: async () => {
    if (!new URLSearchParams(window.location.search).has("demo")) return null;
    window.setTimeout(() => indexSubs.forEach((cb) => cb({ root: "C:\\", files: nodes[0].files, unreadable: 3, refreshing: false, scannedAt: NOW_S })), 50);
    return { root: "C:\\", loaded: true, scannedAt: NOW_S, files: nodes[0].files, unreadable: 3 };
  },
  openRoot: async (root) => {
    await mockApi.scan(root);
    return { root, loaded: false, scannedAt: 0, files: 0, unreadable: 0 };
  },
  scan: async (root) => {
    window.clearInterval(timer);
    let step = 0;
    timer = window.setInterval(() => {
      step++;
      progressSubs.forEach((cb) => cb({ files: step * 41_234, bytes: step * 9.7 * GB, current: `${root}Users\\you\\AppData\\Local\\step${step}` }));
      if (step === 10) {
        window.clearInterval(timer);
        doneSubs.forEach((cb) => cb({ root, ok: true, error: null, cancelled: false, files: nodes[0].files, unreadable: 3, elapsedMs: 1000 }));
      }
    }, 100);
  },
  cancelScan: async () => window.clearInterval(timer),
  getNode: async (id) => view(id),
  nodeByPath: async (path) => {
    const id = nodes.findIndex((_, i) => samePath(pathOf(i), path));
    return id === -1 ? null : id;
  },
  nearestNode: async (path) => {
    let best = 0;
    nodes.forEach((_, i) => {
      if (isWithin(path, pathOf(i)) && pathOf(i).length > pathOf(best).length) best = i;
    });
    return best;
  },
  search: async (query) => mockSearch(query),
  biggest: async (filter) => {
    const started = performance.now();
    const ids = nodes
      .map((_, id) => id)
      .filter((id) => {
        const n = nodes[id];
        return (
          !n.isDir &&
          n.alloc >= (filter.minSize ?? 0) &&
          (filter.olderThanDays === null || n.modified < NOW - filter.olderThanDays * DAY) &&
          (filter.category === null || n.category === filter.category)
        );
      });
    return top(ids, 200, started);
  },
  revealInExplorer: async (path) => console.info("reveal", path),
  openPath: async (path) => console.info("open", path),
  systemData: async (refresh) => {
    if (refresh || !systemStarted) systemStarted = Date.now();
    const elapsed = Date.now() - systemStarted;
    return SYSTEM_ITEMS.map((item, i) =>
      elapsed > 300 + i * 120 ? item : { ...item, state: "pending" as const, alloc: 0, files: 0, path: null, note: null },
    );
  },
  isAdmin: async () => false,
  relaunchAsAdmin: async () => console.info("relaunch as admin"),
  cleanupPlan: async () => CLEAN_ITEMS,
  cleanupStart: async (ids) => {
    const chosen = CLEAN_ITEMS.filter((i) => ids.includes(i.id));
    cleanStatus = { running: true, finished: false, total: chosen.length, done: 0, current: null, moved: 0, inUse: 0, inUseExamples: [], appsOpen: [], refused: [] };
    chosen.forEach((item, i) => {
      window.setTimeout(() => {
        const last = i === chosen.length - 1;
        cleanStatus = {
          ...cleanStatus,
          done: i + 1,
          current: last ? null : chosen[i + 1].roots[0] ?? null,
          moved: cleanStatus.moved + item.alloc * (item.state === "partial" ? 0.9 : 1),
          inUse: cleanStatus.inUse + (item.state === "partial" ? 12 : 0),
          running: !last,
          finished: last,
        };
      }, 400 * (i + 1));
    });
  },
  cleanupStatus: async () => cleanStatus,
  openRecycleBin: async () => console.info("open recycle bin"),
  appsList: async () => ({ loading: false, apps: APPS.filter((a) => !uninstalls.has(a.id) || Date.now() < (uninstalls.get(a.id) ?? 0)) }),
  appUninstall: async (id) => {
    uninstalls.set(id, Date.now() + 1500);
  },
  appUninstallStatus: async (id) => {
    const doneAt = uninstalls.get(id);
    if (doneAt === undefined) return null;
    return { phase: Date.now() < doneAt ? "running" : "done", message: null };
  },
  appStopWaiting: async (id) => {
    uninstalls.delete(id);
  },
  appLeftovers: async (id) => {
    const app = APPS.find((a) => a.id === id);
    if (!app || app.kind === "store") return [];
    return [...(app.installLocation ? [{ path: app.installLocation, alloc: 40 * MB }] : []), ...app.dataFolders];
  },
  recycleLeftovers: async (paths) => ({ moved: paths.length * 300 * MB, left: [], refused: [] }),
  toolsList: async () =>
    tools.map((t) => {
      const done = (toolRuns.get(t.id) ?? Infinity) < Date.now();
      if (t.id === "hibernation" && done) return { ...t, available: false };
      if (t.id === "compactOs" && done) return { ...t, available: false, on: true };
      return t;
    }),
  toolRun: async (id) => {
    toolRuns.set(id, Date.now() + 1500);
  },
  vdisks: async () => DISKS,
  historyList: async () => ({
    root: "C:\\",
    currentAt: NOW_S - HOUR,
    currentTotal: 84.49 * GB,
    points: [
      { scannedAt: NOW_S - 6 * 24 * HOUR, total: 72.2 * GB },
      { scannedAt: NOW_S - 20 * 24 * HOUR, total: 69.8 * GB },
    ],
  }),
  historyDiff: async (scannedAt) => ({ ...HISTORY_DIFF, beforeAt: scannedAt }),
  projectsList: async () =>
    PROJECTS.map((p) => ({ ...p, folders: p.folders.filter((f) => !deleted.has(f.path)) }))
      .filter((p) => p.folders.length > 0)
      .map((p) => ({ ...p, total: p.folders.reduce((s, f) => s + f.alloc, 0) })),
  projectsDelete: async (paths) => {
    const sizes = PROJECTS.flatMap((p) => p.folders).filter((f) => paths.includes(f.path));
    deleteStatus = { running: true, finished: false, total: sizes.length, done: 0, current: sizes[0]?.path ?? null, freed: 0, failed: [], refused: [] };
    sizes.forEach((f, i) =>
      window.setTimeout(() => {
        deleted = new Set([...deleted, f.path]);
        const last = i === sizes.length - 1;
        deleteStatus = { ...deleteStatus, done: i + 1, freed: deleteStatus.freed + f.alloc, current: last ? null : sizes[i + 1].path, running: !last, finished: last };
      }, 300 * (i + 1)),
    );
  },
  projectsStatus: async () => deleteStatus,
  vdiskCompact: async (paths) => {
    compactStarted = Date.now();
    compactPaths = paths;
  },
  vdiskStatus: async (): Promise<CompactStatus | null> => {
    if (!compactStarted) return null;
    const t = Date.now() - compactStarted;
    const phase = t < 600 ? "trimming" : t < 1200 ? "shuttingDown" : t < 2400 ? "compacting" : "done";
    const results = phase === "done" ? DISKS.filter((d) => compactPaths.includes(d.path)).map((d) => ({ path: d.path, before: d.alloc, after: d.alloc * 0.62 })) : [];
    return { phase, message: null, results };
  },
  toolStatus: async (id) => {
    const doneAt = toolRuns.get(id);
    return doneAt === undefined ? null : { phase: Date.now() < doneAt ? "running" : "done", message: null };
  },
  onScanProgress: async (cb) => {
    progressSubs.add(cb);
    return () => progressSubs.delete(cb);
  },
  onScanDone: async (cb) => {
    doneSubs.add(cb);
    return () => doneSubs.delete(cb);
  },
  onIndexUpdated: async (cb: (u: IndexUpdated) => void) => {
    indexSubs.add(cb);
    return () => indexSubs.delete(cb);
  },
};
