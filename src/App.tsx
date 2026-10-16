import { useCallback, useEffect, useRef, useState, type MouseEvent } from "react";
import { api } from "./api";
import { AppsView } from "./components/AppsView";
import { BiggestView } from "./components/BiggestView";
import { ChangesView } from "./components/ChangesView";
import { CleanupView } from "./components/CleanupView";
import { ContextMenu, type MenuTarget } from "./components/ContextMenu";
import { ExploreView } from "./components/ExploreView";
import { ProjectsView } from "./components/ProjectsView";
import { ScanProgressView } from "./components/ScanProgressView";
import { SearchBox } from "./components/SearchBox";
import { SearchView } from "./components/SearchView";
import { SystemDataView } from "./components/SystemDataView";
import { Sidebar } from "./components/Sidebar";
import { Tabs, type TabKey } from "./components/Tabs";
import { TopBar } from "./components/TopBar";
import { Welcome } from "./components/Welcome";
import { displayName, isWithin, joinPath, samePath } from "./paths";
import { S } from "./strings";
import { isMeasuring } from "./systemData";
import type { ChildView, Drive, Hit, NodeView, Opened, Place, ScanProgress, SearchResult, SystemItem } from "./types";

type ScanState =
  | { status: "idle" }
  /** Loading a saved index (root is empty while we don't know which yet). */
  | { status: "opening"; root: string }
  | { status: "scanning"; root: string; progress: ScanProgress | null }
  | { status: "ready"; root: string; unreadable: number }
  | { status: "error"; root: string; message: string };

interface SearchState {
  result: SearchResult | null;
  error: string | null;
}

const SEARCH_DELAY_MS = 150;

export default function App() {
  const [drives, setDrives] = useState<Drive[]>([]);
  const [places, setPlaces] = useState<Place[]>([]);
  const [scan, setScan] = useState<ScanState>({ status: "opening", root: "" });
  const [node, setNode] = useState<NodeView | null>(null);
  /** Folders to go back to, as paths: node ids change when a scan replaces the index. */
  const [history, setHistory] = useState<string[]>([]);
  // `?tab=` only matters in the browser preview (README screenshots); the app has no query string.
  const [tab, setTab] = useState<TabKey>(() => (new URLSearchParams(window.location.search).get("tab") as TabKey | null) ?? "explore");
  const [menu, setMenu] = useState<MenuTarget | null>(null);
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState<SearchState>({ result: null, error: null });
  /** Bumped on every index change so views refetch. */
  const [version, setVersion] = useState(0);
  const [refreshing, setRefreshing] = useState(false);
  const [systemItems, setSystemItems] = useState<SystemItem[] | null>(null);
  const [systemError, setSystemError] = useState<string | null>(null);
  const [elevated, setElevated] = useState(false);
  const nodeRef = useRef<NodeView | null>(null);
  /** Root we're showing or waiting for; events for any other root are stale. */
  const scanRootRef = useRef<string | null>(null);
  /** Folder to open once the scan finishes (rescans keep you where you were). */
  const focusRef = useRef<string | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const searchSeq = useRef(0);

  const ready = scan.status === "ready";

  const showNode = useCallback(async (id: number, pushHistory: boolean) => {
    const view = await api.getNode(id);
    const prev = nodeRef.current;
    if (pushHistory && prev && prev.id !== id) setHistory((h) => [...h, prev.path]);
    nodeRef.current = view;
    setNode(view);
  }, []);

  const refreshDrives = useCallback(() => {
    api.listDrives().then(setDrives).catch(console.error);
  }, []);

  /** Shows a freshly loaded or scanned index, at `focus` if it is still there. */
  const enterReady = useCallback(
    async (root: string, unreadable: number, focus: string | null) => {
      setScan({ status: "ready", root, unreadable });
      nodeRef.current = null;
      setHistory([]);
      const id = focus ? await api.nearestNode(focus) : 0;
      await showNode(id, false);
      setVersion((v) => v + 1);
      refreshDrives();
    },
    [showNode, refreshDrives],
  );

  /** Handles the answer to `restore` / `openRoot`. */
  const opened = useCallback(
    (o: Opened) => {
      scanRootRef.current = o.root;
      if (o.loaded) {
        setRefreshing(true);
        enterReady(o.root, o.unreadable, null).catch(console.error);
      } else {
        setScan((s) => (s.status === "opening" ? { status: "scanning", root: o.root, progress: null } : s));
      }
    },
    [enterReady],
  );

  useEffect(() => {
    refreshDrives();
    api.listPlaces().then(setPlaces).catch(console.error);
    api
      .restore()
      .then((o) => (o ? opened(o) : setScan({ status: "idle" })))
      .catch((e) => {
        console.error(e);
        setScan({ status: "idle" });
      });
  }, [refreshDrives, opened]);

  useEffect(() => {
    const subscriptions = [
      api.onScanProgress((progress) =>
        setScan((s) =>
          s.status === "scanning" || (s.status === "opening" && s.root)
            ? { status: "scanning", root: s.root, progress }
            : s,
        ),
      ),
      api.onScanDone(async (done) => {
        if (done.cancelled || !scanRootRef.current || !samePath(done.root, scanRootRef.current)) return;
        setRefreshing(false);
        if (!done.ok) {
          setScan({ status: "error", root: done.root, message: done.error ?? S.scanFailed });
          return;
        }
        const focus = focusRef.current;
        focusRef.current = null;
        await enterReady(done.root, done.unreadable, focus);
      }),
      api.onIndexUpdated(async (u) => {
        if (!scanRootRef.current || !samePath(u.root, scanRootRef.current)) return;
        setRefreshing(u.refreshing);
        setScan((s) => (s.status === "ready" ? { ...s, unreadable: u.unreadable } : s));
        const current = nodeRef.current;
        if (current) {
          const id = await api.nearestNode(current.path);
          await showNode(id, false);
        }
        setVersion((v) => v + 1);
      }),
    ];
    return () => subscriptions.forEach((p) => p.then((unlisten) => unlisten()));
  }, [showNode, enterReady]);

  // Ctrl+F focuses the search box (and keeps WebView2's find bar away).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const loadSystemData = useCallback((refresh: boolean) => {
    api
      .systemData(refresh)
      .then((items) => {
        setSystemItems(items);
        setSystemError(null);
      })
      .catch((e) => setSystemError(String(e)));
  }, []);

  useEffect(() => {
    api.isAdmin().then(setElevated).catch(console.error);
  }, []);

  /** After a clean-up, sizes changed: measure System Data again. */
  const remeasureSystemData = useCallback(() => loadSystemData(true), [loadSystemData]);

  // Measure the first time the tab opens, then poll while rows are pending.
  const systemMeasuring = tab === "systemData" && !systemError && isMeasuring(systemItems);
  useEffect(() => {
    if (!systemMeasuring) return;
    loadSystemData(false);
    const timer = window.setInterval(() => loadSystemData(false), 400);
    return () => window.clearInterval(timer);
  }, [systemMeasuring, loadSystemData]);

  useEffect(() => {
    const seq = ++searchSeq.current;
    if (!ready || !query.trim()) {
      setSearch({ result: null, error: null });
      return;
    }
    const timer = window.setTimeout(() => {
      api
        .search(query)
        .then((result) => seq === searchSeq.current && setSearch({ result, error: null }))
        .catch((e) => seq === searchSeq.current && setSearch((s) => ({ result: s.result, error: String(e) })));
    }, SEARCH_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [query, version, ready]);

  const leave = () => {
    nodeRef.current = null;
    setNode(null);
    setHistory([]);
    setQuery("");
    setRefreshing(false);
  };

  /** Opens a drive or folder: from its saved index when there is one, else by scanning. */
  const openRoot = (root: string) => {
    scanRootRef.current = root;
    focusRef.current = null;
    leave();
    setScan({ status: "opening", root });
    api
      .openRoot(root)
      .then((o) => scanRootRef.current === root && opened(o))
      .catch((e) => setScan({ status: "error", root, message: String(e) }));
  };

  /** Scans `root` from scratch, with the progress screen. */
  const startScan = (root: string, focus: string | null = null) => {
    scanRootRef.current = root;
    leave();
    focusRef.current = focus;
    setScan({ status: "scanning", root, progress: null });
    api.scan(root).catch((e) => setScan({ status: "error", root, message: String(e) }));
  };

  const cancelScan = () => {
    scanRootRef.current = null;
    api.cancelScan().catch(console.error);
    setScan({ status: "idle" });
  };

  const openDir = (id: number) => {
    setQuery("");
    setTab("explore");
    showNode(id, true).catch(console.error);
  };

  const openPlace = (path: string) => {
    if (scan.status === "ready" && isWithin(path, scan.root)) {
      api
        .nodeByPath(path)
        .then((id) => (id === null ? openRoot(path) : openDir(id)))
        .catch(console.error);
    } else {
      openRoot(path);
    }
  };

  const goBack = () => {
    const prev = history[history.length - 1];
    if (prev === undefined) return;
    setHistory((h) => h.slice(0, -1));
    api
      .nearestNode(prev)
      .then((id) => showNode(id, false))
      .catch(console.error);
  };

  const openMenu = (e: MouseEvent, child: ChildView) => {
    e.preventDefault();
    if (!node) return;
    setMenu({ x: e.clientX, y: e.clientY, id: child.id, isDir: child.isDir, path: joinPath(node.path, child.name) });
  };

  /** A search or Biggest row: folders open in Explore, files show their folder. */
  const openHit = (h: Hit) => {
    api
      .nearestNode(h.isDir ? joinPath(h.parent, h.name) : h.parent)
      .then(openDir)
      .catch(console.error);
  };
  const openHitFile = (h: Hit) => api.openPath(joinPath(h.parent, h.name)).catch(console.error);
  const hitMenu = (e: MouseEvent, h: Hit) => {
    e.preventDefault();
    setMenu({ x: e.clientX, y: e.clientY, id: h.id, isDir: h.isDir, path: joinPath(h.parent, h.name) });
  };

  const openInExplore = (path: string) => {
    setQuery("");
    setTab("explore");
    openPlace(path);
  };
  const runAsAdmin = () => {
    api.relaunchAsAdmin().catch(console.error);
  };

  const scanRoot = scan.status === "idle" || !scan.root ? null : scan.root;

  let content;
  if (tab === "projects" && !query.trim()) {
    content = <ProjectsView version={version} />;
  } else if (tab === "changes" && !query.trim()) {
    content = <ChangesView version={version} onOpen={openInExplore} />;
  } else if (tab === "apps" && !query.trim()) {
    content = <AppsView />;
  } else if (tab === "cleanup" && !query.trim()) {
    content = <CleanupView onCleaned={remeasureSystemData} />;
  } else if (tab === "systemData" && !query.trim()) {
    content = (
      <SystemDataView
        items={systemItems}
        error={systemError}
        elevated={elevated}
        onRefresh={() => loadSystemData(true)}
        onRunAsAdmin={runAsAdmin}
        onOpenInExplore={openInExplore}
        onReveal={(path) => api.revealInExplorer(path).catch(console.error)}
      />
    );
  } else if (scan.status === "idle") {
    content = <Welcome drives={drives} onScan={openRoot} />;
  } else if (scan.status === "opening") {
    content = (
      <div className="center">
        <div>
          <div className="spinner" />
          {scan.root && <p className="muted">{S.opening(displayName(scan.root))}</p>}
        </div>
      </div>
    );
  } else if (scan.status === "scanning") {
    content = <ScanProgressView root={scan.root} progress={scan.progress} onCancel={cancelScan} />;
  } else if (scan.status === "error") {
    const root = scan.root;
    content = (
      <div className="center">
        <div className="card panel">
          <h2>{S.scanFailed}</h2>
          <p className="muted">{scan.message}</p>
          <button className="btn" onClick={() => startScan(root)}>
            {S.tryAgain}
          </button>
        </div>
      </div>
    );
  } else if (query.trim()) {
    content = (
      <SearchView
        result={search.result}
        error={search.error}
        onOpen={openHit}
        onOpenFile={openHitFile}
        onMenu={hitMenu}
      />
    );
  } else if (tab === "biggest") {
    content = <BiggestView version={version} onOpen={openHit} onOpenFile={openHitFile} onMenu={hitMenu} />;
  } else if (node) {
    content = (
      <ExploreView
        node={node}
        unreadable={scan.unreadable}
        onOpenDir={openDir}
        onOpenFile={(name) => api.openPath(joinPath(node.path, name)).catch(console.error)}
        onMenu={openMenu}
      />
    );
  }

  return (
    <div className="app" onContextMenu={(e) => e.preventDefault()}>
      <Sidebar
        drives={drives}
        places={places}
        activePath={node?.path ?? scanRoot}
        onDrive={(d) => openRoot(d.root)}
        onPlace={openPlace}
        elevated={elevated}
        onRunAsAdmin={runAsAdmin}
      />
      <main className="main">
        <TopBar
          crumbs={node?.crumbs ?? []}
          canGoBack={history.length > 0}
          refreshing={refreshing}
          search={<SearchBox value={query} disabled={!ready} inputRef={searchInputRef} onChange={setQuery} />}
          onBack={goBack}
          onCrumb={openDir}
          onRescan={scanRoot && scan.status !== "scanning" ? () => startScan(scanRoot, node?.path ?? null) : undefined}
        />
        <Tabs
          active={query.trim() && ready ? null : tab}
          onChange={(t) => {
            setQuery("");
            setTab(t);
          }}
        />
        {content}
      </main>
      {menu && <ContextMenu target={menu} onClose={() => setMenu(null)} onOpenDir={openDir} />}
    </div>
  );
}
