import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";
import { WINDOWS_KEY, appName, filterApps, formatPercent, sortApps, stableOrder, type SortKey } from "../activity";
import { formatBytes } from "../format";
import { useSessionState } from "../sessionState";
import { S } from "../strings";
import type { ActivityApp, ActivityProcess, ActivitySnapshot, EndResult } from "../types";
import { LoadError } from "./LoadError";
import { Sparkline } from "./Sparkline";

const POLL_MS = 1000;
/** Rows re-sort this often, so they don't jump under the pointer every second. */
const RESORT_MS = 3000;

type Selection = { key: string; pid: number | null };

interface Notice {
  text: string;
  tone: "ok" | "warn";
  /** Offer Run as administrator. */
  elevate: boolean;
}

interface Props {
  elevated: boolean;
  onRunAsAdmin: () => void;
}

export function ActivityView({ elevated, onRunAsAdmin }: Props) {
  const [snap, setSnap] = useState<ActivitySnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sort, setSort] = useSessionState<SortKey>("activity.sort", "cpu");
  const [query, setQuery] = useState("");
  const [order, setOrder] = useState<string[]>([]);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [selected, setSelected] = useState<Selection | null>(null);
  const [confirming, setConfirming] = useState<ActivityApp | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const snapRef = useRef<ActivitySnapshot | null>(null);

  useEffect(() => {
    let alive = true;
    const poll = () =>
      api
        .activitySnapshot()
        .then((s) => {
          if (!alive || !s) return;
          snapRef.current = s;
          setSnap(s);
          setError(null);
        })
        .catch((e) => alive && setError(String(e)));
    poll();
    const timer = window.setInterval(poll, POLL_MS);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);

  // Re-sort on a slower beat than the numbers, and right away when the column changes.
  useEffect(() => {
    const resort = () => {
      const s = snapRef.current;
      if (s) setOrder(sortApps(s.apps, sort).map((a) => a.key));
    };
    resort();
    const timer = window.setInterval(resort, RESORT_MS);
    return () => window.clearInterval(timer);
  }, [sort, snap === null]);

  const rows = useMemo(() => (snap ? stableOrder(filterApps(snap.apps, query), order) : []), [snap, query, order]);

  // Polling goes on, so a retry only has to clear the message.
  if (error) return <LoadError message={error} onRetry={() => setError(null)} />;
  if (!snap) {
    return (
      <div className="center">
        <div>
          <div className="spinner" />
          <p className="muted">{S.measuringActivity}</p>
        </div>
      </div>
    );
  }

  const { system } = snap;
  const selectedApp = selected ? snap.apps.find((a) => a.key === selected.key) ?? null : null;
  // A process row, or an app that is a single process: either way there is one PID to end.
  let selectedProc: ActivityProcess | null = null;
  if (selectedApp && selected) {
    selectedProc =
      selected.pid !== null
        ? (selectedApp.processes.find((p) => p.pid === selected.pid) ?? null)
        : selectedApp.processes.length === 1
          ? selectedApp.processes[0]
          : null;
  }

  const report = (results: EndResult[]) => {
    const count = (o: EndResult["outcome"]) => results.filter((r) => r.outcome === o).length;
    const ended = count("ended");
    if (count("denied") > 0) setNotice({ text: S.endDenied, tone: "warn", elevate: !elevated });
    else if (count("refused") > 0) setNotice({ text: S.endRefused, tone: "warn", elevate: false });
    else if (count("failed") > 0) setNotice({ text: S.endFailed, tone: "warn", elevate: false });
    else if (ended > 0) setNotice({ text: S.ended(ended), tone: "ok", elevate: false });
    else setNotice({ text: S.endGone, tone: "ok", elevate: false });
    setSelected(null);
  };
  const end = (pids: number[]) => {
    setConfirming(null);
    api.activityEnd(pids).then(report).catch((e) => setNotice({ text: String(e), tone: "warn", elevate: false }));
  };

  const toggle = (key: string) =>
    setExpanded((e) => {
      const next = new Set(e);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  const header = (key: SortKey, label: string) => (
    <button className={`act-col${sort === key ? " sorted" : ""}`} onClick={() => setSort(key)}>
      {label}
    </button>
  );

  return (
    <div className="activity">
      <div className="sysdata-head">
        <div>
          <h3>{S.activity}</h3>
          <p className="muted small">{S.activityBody}</p>
        </div>
      </div>

      <div className="act-stats">
        <Stat
          label={S.cpu}
          value={formatPercent(system.cpu)}
          detail={S.cores(system.cores)}
          values={snap.history.map((p) => p.cpu)}
          max={100}
          color="var(--accent)"
        />
        <Stat
          label={S.memory}
          value={formatPercent((system.ramUsed / Math.max(system.ramTotal, 1)) * 100)}
          detail={S.ofTotal(formatBytes(system.ramUsed), formatBytes(system.ramTotal))}
          values={snap.history.map((p) => p.ramUsed)}
          max={system.ramTotal}
          color="var(--cat-documents)"
        />
        <Stat
          label={S.gpu}
          value={formatPercent(system.gpu)}
          detail={
            system.gpuMemUsed === null
              ? S.noGpuCounters
              : S.gpuMemory(formatBytes(system.gpuMemUsed), system.gpuMemTotal === null ? null : formatBytes(system.gpuMemTotal))
          }
          values={snap.history.map((p) => p.gpu)}
          max={100}
          color="var(--cat-images)"
        />
      </div>

      <div className="act-bar">
        <input
          className="act-filter"
          placeholder={S.searchActivity}
          value={query}
          spellCheck={false}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="spacer" />
        {notice && (
          <div className={`act-notice ${notice.tone}`}>
            <span>{notice.text}</span>
            {notice.elevate && (
              <button className="btn compact-btn" onClick={onRunAsAdmin}>
                {S.runAsAdmin}
              </button>
            )}
            <button className="act-dismiss" aria-label={S.cancel} onClick={() => setNotice(null)}>
              ×
            </button>
          </div>
        )}
        <EndButton
          app={selectedApp}
          proc={selectedProc}
          onEndProcess={(p) => end([p.pid])}
          onEndApp={(a) => setConfirming(a)}
        />
      </div>

      <div className="card act-table" role="table">
        <div className="act-row act-head" role="row">
          <span className="act-col name">{S.name}</span>
          {header("cpu", S.cpu)}
          {header("ram", S.memory)}
          {header("gpu", S.gpu)}
        </div>
        <div className="act-body">
          {rows.map((app) => {
            const many = app.processes.length > 1;
            const open = many && expanded.has(app.key);
            const isSelected = selected?.key === app.key && (selected.pid === null || !many);
            return (
              <div key={app.key} role="rowgroup">
                <div
                  className={`act-row app${isSelected ? " selected" : ""}`}
                  role="row"
                  onClick={() => setSelected({ key: app.key, pid: null })}
                  onDoubleClick={() => many && toggle(app.key)}
                >
                  <span className="act-cell name">
                    {many ? (
                      <button
                        className={`act-chevron${open ? " open" : ""}`}
                        aria-label={appName(app)}
                        aria-expanded={open}
                        onClick={(e) => {
                          e.stopPropagation();
                          toggle(app.key);
                        }}
                      >
                        ›
                      </button>
                    ) : (
                      <span className="act-chevron-space" />
                    )}
                    <span className="act-name">{appName(app)}</span>
                    {many && <span className="muted small">({app.processes.length})</span>}
                  </span>
                  <Metric value={app.cpu} heat={app.cpu} text={formatPercent(app.cpu)} />
                  <Metric value={app.ram} heat={(app.ram / Math.max(system.ramTotal, 1)) * 400} text={formatBytes(app.ram)} />
                  <Metric value={app.gpu} heat={app.gpu ?? 0} text={formatPercent(app.gpu)} />
                </div>
                {open &&
                  [...app.processes]
                    .sort((a, b) => (b[sort] ?? -1) - (a[sort] ?? -1) || a.pid - b.pid)
                    .map((p) => (
                      <div
                        key={p.pid}
                        className={`act-row proc${selected?.pid === p.pid ? " selected" : ""}`}
                        role="row"
                        onClick={() => setSelected({ key: app.key, pid: p.pid })}
                      >
                        <span className="act-cell name">
                          <span className="act-name">{p.name}</span>
                          <span className="muted small">{p.pid}</span>
                        </span>
                        <Metric value={p.cpu} heat={p.cpu} text={formatPercent(p.cpu)} />
                        <Metric value={p.ram} heat={(p.ram / Math.max(system.ramTotal, 1)) * 400} text={formatBytes(p.ram)} />
                        <Metric value={p.gpu} heat={p.gpu ?? 0} text={formatPercent(p.gpu)} />
                      </div>
                    ))}
              </div>
            );
          })}
        </div>
      </div>

      {confirming && (
        <div className="modal-backdrop" onClick={() => setConfirming(null)}>
          <div className="card modal" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
            <h3>{S.endAppTitle(appName(confirming))}</h3>
            <p className="muted">{S.endAppBody(confirming.processes.length)}</p>
            <div className="modal-actions">
              <button className="btn" onClick={() => setConfirming(null)} autoFocus>
                {S.cancel}
              </button>
              <button className="btn primary danger" onClick={() => end(confirming.processes.map((p) => p.pid))}>
                {S.endApp}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Stat(props: { label: string; value: string; detail: string; values: (number | null)[]; max: number; color: string }) {
  return (
    <div className="card act-stat">
      <div className="act-stat-head">
        <span className="act-stat-label">
          <span className="dot" style={{ background: props.color }} />
          {props.label}
        </span>
        <span className="act-stat-value">{props.value}</span>
      </div>
      <Sparkline values={props.values} max={props.max} color={props.color} />
      <div className="muted small">{props.detail}</div>
    </div>
  );
}

/** A number cell tinted by how much it uses, like Task Manager's heat map. */
function Metric({ value, heat, text }: { value: number | null; heat: number; text: string }) {
  const tint = value === null ? 0 : Math.min(heat, 100) * 0.45;
  return (
    <span className="act-cell metric" style={{ background: `color-mix(in srgb, var(--accent) ${tint.toFixed(1)}%, transparent)` }}>
      {text}
    </span>
  );
}

function EndButton(props: {
  app: ActivityApp | null;
  proc: ActivityProcess | null;
  onEndProcess: (p: ActivityProcess) => void;
  onEndApp: (a: ActivityApp) => void;
}) {
  const { app, proc } = props;
  if (proc) {
    return (
      <button className="btn act-end" onClick={() => props.onEndProcess(proc)}>
        {S.endProcess}
      </button>
    );
  }
  if (app && app.key === WINDOWS_KEY) {
    return (
      <button className="btn act-end" disabled title={S.windowsGroupNote}>
        {S.endApp}
      </button>
    );
  }
  return (
    <button className="btn act-end" disabled={!app} onClick={() => app && props.onEndApp(app)}>
      {S.endApp}
    </button>
  );
}
