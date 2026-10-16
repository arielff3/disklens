import { useEffect, useState } from "react";
import { api } from "../api";
import { formatBytes } from "../format";
import { S } from "../strings";
import { clearSessionState, useSessionState } from "../sessionState";
import type { CompactStatus, VDisk } from "../types";

const POLL_MS = 800;

interface Props {
  onClose: (changed: boolean) => void;
}

/** Lists WSL and Docker disks and compacts the chosen ones. */
export function CompactDialog({ onClose: close }: Props) {
  const onClose = (changed: boolean) => {
    clearSessionState("compact.status");
    close(changed);
  };
  const [disks, setDisks] = useState<VDisk[] | null>(null);
  const [chosen, setChosen] = useState<Set<string>>(new Set());
  const [status, setStatus] = useSessionState<CompactStatus | null>("compact.status", null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .vdisks()
      .then((d) => {
        setDisks(d);
        setChosen(new Set(d.map((x) => x.path)));
      })
      .catch((e) => setError(String(e)));
  }, []);

  const working = status !== null && status.phase !== "done" && status.phase !== "failed";
  useEffect(() => {
    if (!working) return;
    const timer = window.setInterval(() => api.vdiskStatus().then(setStatus).catch(console.error), POLL_MS);
    return () => window.clearInterval(timer);
  }, [working, setStatus]);

  const start = () => {
    setError(null);
    api
      .vdiskCompact([...chosen])
      .then(() => setStatus({ phase: "trimming", message: null, results: [] }))
      .catch((e) => setError(String(e)));
  };

  const toggle = (path: string) =>
    setChosen((c) => {
      const next = new Set(c);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });

  const finished = status?.phase === "done" || status?.phase === "failed";
  const label = (path: string) => {
    const disk = disks?.find((d) => d.path === path);
    return disk?.distro ?? (disk?.kind === "docker" ? "Docker" : path.split("\\").slice(-3, -1).join("\\"));
  };

  return (
    <div className="modal-backdrop" onMouseDown={() => !working && onClose(finished)}>
      <div className="card modal" role="dialog" aria-modal="true" onMouseDown={(e) => e.stopPropagation()}>
        <h3>{S.compactTitle}</h3>
        <p className="muted small">{S.compactBody}</p>

        {!status && (
          <>
            {disks?.length === 0 && <div className="empty">{S.noDisks}</div>}
            {disks && disks.length > 0 && (
              <div className="confirm-list">
                {disks.map((d) => (
                  <label key={d.path} className="leftover-row">
                    <input type="checkbox" checked={chosen.has(d.path)} onChange={() => toggle(d.path)} />
                    <span className="clean-text">
                      <span className="name">{d.distro ?? (d.kind === "docker" ? "Docker" : "WSL")}</span>
                      <span className="confirm-path" title={d.path}>
                        {d.path}
                      </span>
                    </span>
                    <span className="size">{formatBytes(d.alloc)}</span>
                  </label>
                ))}
              </div>
            )}
            <div className="notice">{S.compactWarning}</div>
          </>
        )}

        {status && !finished && (
          <div className="compact-progress">
            <div className="spinner" />
            <p>{S.compactPhases[status.phase]}</p>
          </div>
        )}

        {finished && status && (
          <>
            <h3>{S.compactPhases[status.phase]}</h3>
            {status.message && <div className="notice">{status.message}</div>}
            <div className="confirm-list">
              {status.results.map((r) => (
                <div key={r.path} className="confirm-head">
                  <span className="name">{label(r.path)}</span>
                  <span className="muted small">
                    {formatBytes(r.before)} → {formatBytes(r.after)}
                  </span>
                  <span className="size">{S.compactSaved(formatBytes(Math.max(0, r.before - r.after)))}</span>
                </div>
              ))}
            </div>
          </>
        )}

        {error && <div className="notice">{error}</div>}

        <div className="modal-actions">
          {!status && (
            <>
              <button className="btn" onClick={() => onClose(false)}>
                {S.cancel}
              </button>
              <button className="btn primary" disabled={chosen.size === 0} onClick={start}>
                {S.compactButton(chosen.size)}
              </button>
            </>
          )}
          {finished && (
            <button className="btn primary" onClick={() => onClose(true)} autoFocus>
              {S.close}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
