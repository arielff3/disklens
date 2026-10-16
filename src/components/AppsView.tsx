import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { avatarTile, footprint, isMeasuringApps, visibleApps } from "../apps";
import { formatBytes, splitBytes } from "../format";
import { RefreshIcon, RevealIcon } from "../icons";
import { S } from "../strings";
import type { AppsList, AppView, FolderSize, RecycleReport, UninstallStatus } from "../types";
import { useSessionState } from "../sessionState";
import { LoadError } from "./LoadError";

const POLL_MS = 500;

/** Where an uninstall stands, shown as a dialog. */
type Flow =
  | { step: "confirm"; app: AppView }
  | { step: "waiting"; app: AppView }
  | { step: "leftovers"; app: AppView; folders: FolderSize[]; chosen: Set<string> }
  | { step: "message"; title: string; body?: string };

export function AppsView() {
  const [list, setList] = useState<AppsList | null>(null);
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<string | null>(null);
  const [flow, setFlow] = useSessionState<Flow | null>("apps.flow", null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const load = useCallback((refresh: boolean) => {
    api
      .appsList(refresh)
      .then((l) => {
        setList(l);
        setLoadError(null);
      })
      .catch((e) => setLoadError(String(e)));
  }, []);

  useEffect(() => load(false), [load]);

  const measuring = !loadError && isMeasuringApps(list);
  useEffect(() => {
    if (!measuring) return;
    const timer = window.setInterval(() => load(false), POLL_MS);
    return () => window.clearInterval(timer);
  }, [measuring, load]);

  // Follow a running uninstall until it is done, failed or stopped.
  const waitingFor = flow?.step === "waiting" ? flow.app : null;
  useEffect(() => {
    if (!waitingFor) return;
    const timer = window.setInterval(() => {
      api
        .appUninstallStatus(waitingFor.id)
        .then((status) => {
          if (status && status.phase !== "running") void finished(waitingFor, status);
        })
        .catch(console.error);
    }, POLL_MS * 2);
    return () => window.clearInterval(timer);
  }, [waitingFor]);

  const finished = async (app: AppView, status: UninstallStatus) => {
    load(false);
    if (status.phase === "failed") {
      setFlow({ step: "message", title: S.uninstallFailed, body: status.message ?? undefined });
    } else if (status.phase === "stopped") {
      setFlow({ step: "message", title: S.stoppedWaiting });
    } else {
      const folders = app.kind === "program" ? await api.appLeftovers(app.id) : [];
      setFlow(
        folders.length > 0
          ? { step: "leftovers", app, folders, chosen: new Set(folders.map((f) => f.path)) }
          : { step: "message", title: S.uninstalled(app.name) },
      );
    }
  };

  const startUninstall = (app: AppView) => {
    setFlow({ step: "waiting", app });
    api.appUninstall(app.id).catch((e) => setFlow({ step: "message", title: S.uninstallFailed, body: String(e) }));
  };

  const recycle = (folders: FolderSize[], chosen: Set<string>) => {
    api
      .recycleLeftovers(folders.filter((f) => chosen.has(f.path)).map((f) => f.path))
      .then((report: RecycleReport) =>
        setFlow({
          step: "message",
          title: S.recycled(formatBytes(report.moved)),
          body: report.left.length > 0 ? S.leftInUse(report.left.length) : S.emptyBinHint,
        }),
      )
      .catch((e) => setFlow({ step: "message", title: S.uninstallFailed, body: String(e) }));
  };

  const apps = visibleApps(list?.apps ?? [], query);

  return (
    <div className="apps">
      <div className="sysdata-head">
        <div>
          <h3>{S.appsTitle}</h3>
          <p className="muted small">{S.appsBody}</p>
        </div>
        <div className="spacer" />
        {measuring && (
          <span className="pill">
            <span className="mini-spinner" />
            {S.measuring}
          </span>
        )}
        <input className="apps-filter" placeholder={S.filterApps} value={query} onChange={(e) => setQuery(e.target.value)} />
        <button className="icon-btn" onClick={() => load(true)} disabled={measuring} title={S.refreshList} aria-label={S.refreshList}>
          <RefreshIcon />
        </button>
      </div>
      {loadError && <LoadError message={loadError} onRetry={() => load(false)} />}
      <div className="card apps-list">
        {list && !list.loading && apps.length === 0 && <div className="empty">{S.noApps}</div>}
        {apps.map((app) => (
          <AppRow
            key={app.id}
            app={app}
            open={open === app.id}
            onToggle={() => setOpen((o) => (o === app.id ? null : app.id))}
            onUninstall={() => setFlow({ step: "confirm", app })}
          />
        ))}
      </div>
      {flow && (
        <FlowDialog
          flow={flow}
          onClose={() => setFlow(null)}
          onConfirm={startUninstall}
          onStopWaiting={(app) => api.appStopWaiting(app.id).catch(console.error)}
          onChoose={(chosen) => flow.step === "leftovers" && setFlow({ ...flow, chosen })}
          onRecycle={recycle}
        />
      )}
    </div>
  );
}

function AppRow({ app, open, onToggle, onUninstall }: { app: AppView; open: boolean; onToggle: () => void; onUninstall: () => void }) {
  const meta = [app.publisher, app.version].filter(Boolean).join(" · ");
  return (
    <div className={`app-row${open ? " open" : ""}`}>
      <button className="app-summary" onClick={onToggle} aria-expanded={open}>
        <span className="app-avatar" style={{ background: `var(--tile-${avatarTile(app.name)})` }}>
          {app.name.trim().charAt(0).toUpperCase()}
        </span>
        <span className="app-name">
          <span className="name">{app.name}</span>
          <span className="muted small">
            {meta}
            {app.kind === "store" && <span className="badge app-badge">{S.storeBadge}</span>}
          </span>
        </span>
        {app.measured ? <Size bytes={footprint(app)} estimated={app.estimated} /> : <div className="pending-bar" aria-label={S.measuring} />}
      </button>
      {open && (
        <div className="app-details">
          {app.installLocation && <Folder label={S.installFolder} path={app.installLocation} bytes={app.installSize} />}
          {!app.installLocation && app.installSize > 0 && (
            <div className="app-folder">
              <span className="muted small">{S.installFolder}</span>
              <span className="muted small">{S.estimated}</span>
              <span className="size">{formatBytes(app.installSize)}</span>
            </div>
          )}
          {app.dataFolders.map((f) => (
            <Folder key={f.path} label={S.dataFolder} path={f.path} bytes={f.alloc} />
          ))}
          <div className="app-actions">
            <button className="btn" onClick={onUninstall}>
              {S.uninstall}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function Folder({ label, path, bytes }: { label: string; path: string; bytes: number }) {
  return (
    <div className="app-folder">
      <span className="muted small">{label}</span>
      <span className="confirm-path" title={path}>
        {path}
      </span>
      <span className="size">{formatBytes(bytes)}</span>
      <button className="icon-btn" onClick={() => api.revealInExplorer(path).catch(console.error)} title={S.reveal} aria-label={S.reveal}>
        <RevealIcon />
      </button>
    </div>
  );
}

function Size({ bytes, estimated }: { bytes: number; estimated: boolean }) {
  const { value, unit } = splitBytes(bytes);
  return (
    <div className="size" title={estimated ? S.estimated : undefined}>
      {estimated ? "~" : ""}
      {value}
      <span className="unit">{unit}</span>
    </div>
  );
}

interface DialogProps {
  flow: Flow;
  onClose: () => void;
  onConfirm: (app: AppView) => void;
  onStopWaiting: (app: AppView) => void;
  onChoose: (chosen: Set<string>) => void;
  onRecycle: (folders: FolderSize[], chosen: Set<string>) => void;
}

function FlowDialog({ flow, onClose, onConfirm, onStopWaiting, onChoose, onRecycle }: DialogProps) {
  const closable = flow.step !== "waiting";
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && closable && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [closable, onClose]);

  let body;
  if (flow.step === "confirm") {
    const { app } = flow;
    body = (
      <>
        <h3>{S.uninstallTitle(app.name)}</h3>
        <p className="muted small">{app.kind === "store" ? S.uninstallStoreBody(app.name) : S.uninstallProgramBody(app.name)}</p>
        <div className="modal-actions">
          <button className="btn" onClick={onClose}>
            {S.cancel}
          </button>
          <button className="btn primary" onClick={() => onConfirm(app)} autoFocus>
            {S.uninstall}
          </button>
        </div>
      </>
    );
  } else if (flow.step === "waiting") {
    body = (
      <>
        <div className="spinner" />
        <h3 className="center-text">{flow.app.kind === "store" ? S.removingStoreApp : S.waitingUninstaller}</h3>
        {flow.app.kind === "program" && (
          <div className="modal-actions center-actions">
            <button className="btn" onClick={() => onStopWaiting(flow.app)}>
              {S.stopWaiting}
            </button>
          </div>
        )}
      </>
    );
  } else if (flow.step === "leftovers") {
    const { folders, chosen } = flow;
    const total = folders.filter((f) => chosen.has(f.path)).reduce((s, f) => s + f.alloc, 0);
    const toggle = (path: string) => {
      const next = new Set(chosen);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      onChoose(next);
    };
    body = (
      <>
        <h3>{S.leftoversTitle(flow.app.name)}</h3>
        <p className="muted small">{S.leftoversBody}</p>
        <div className="confirm-list">
          {folders.map((f) => (
            <label key={f.path} className="leftover-row">
              <input type="checkbox" checked={chosen.has(f.path)} onChange={() => toggle(f.path)} />
              <span className="confirm-path" title={f.path}>
                {f.path}
              </span>
              <span className="size">{formatBytes(f.alloc)}</span>
            </label>
          ))}
        </div>
        <div className="modal-actions">
          <button className="btn" onClick={onClose}>
            {S.keepThem}
          </button>
          <button className="btn primary" disabled={chosen.size === 0} onClick={() => onRecycle(folders, chosen)}>
            {S.moveLeftovers(formatBytes(total))}
          </button>
        </div>
      </>
    );
  } else {
    body = (
      <>
        <h3>{flow.title}</h3>
        {flow.body && <p className="muted small">{flow.body}</p>}
        <div className="modal-actions">
          <button className="btn primary" onClick={onClose} autoFocus>
            {S.ok}
          </button>
        </div>
      </>
    );
  }

  return (
    <div className="modal-backdrop" onMouseDown={() => closable && onClose()}>
      <div className="card modal" role="dialog" aria-modal="true" onMouseDown={(e) => e.stopPropagation()}>
        {body}
      </div>
    </div>
  );
}
