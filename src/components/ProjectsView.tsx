import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { formatBytes, splitBytes } from "../format";
import { defaultSelection, idleDays, selectedTotal } from "../projects";
import { S } from "../strings";
import type { DeleteStatus, Project } from "../types";
import { useSessionState } from "../sessionState";
import { LoadError } from "./LoadError";

const POLL_MS = 400;

type Phase = "choose" | "confirm" | "deleting" | "done";

interface Props {
  /** Bumped when the index changes, so the list is redone. */
  version: number;
}

export function ProjectsView({ version }: Props) {
  const [projects, setProjects] = useState<Project[] | null | undefined>(undefined);
  const [selected, setSelected] = useSessionState<Set<string> | null>("projects.selected", null);
  const [phase, setPhase] = useSessionState<Phase>("projects.phase", "choose");
  const [status, setStatus] = useSessionState<DeleteStatus | null>("projects.status", null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api
      .projectsList()
      .then((p) => {
        setProjects(p);
        setError(null);
        if (p) setSelected((s) => s ?? defaultSelection(p, Date.now() / 1000));
      })
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    if (phase === "choose") load();
  }, [load, version, phase]);

  useEffect(() => {
    if (phase !== "deleting") return;
    const poll = () =>
      api
        .projectsStatus()
        .then((s) => {
          setStatus(s);
          if (s.finished) setPhase("done");
        })
        .catch(console.error);
    poll(); // right away: the job may have finished while another tab was open
    const timer = window.setInterval(poll, POLL_MS);
    return () => window.clearInterval(timer);
  }, [phase, setPhase, setStatus]);

  const list = projects ?? [];
  const chosen = selected ?? new Set<string>();
  const total = selectedTotal(list, chosen);
  const chosenFolders = list.flatMap((p) => p.folders).filter((f) => chosen.has(f.path));

  const toggle = (paths: string[], on: boolean) =>
    setSelected((s) => {
      const next = new Set(s);
      for (const p of paths) {
        if (on) next.add(p);
        else next.delete(p);
      }
      return next;
    });

  const start = () => {
    setStatus(null);
    // Switch to progress only once the job is registered, so the first poll can't see the previous one.
    api
      .projectsDelete(chosenFolders.map((f) => f.path))
      .then(() => setPhase("deleting"))
      .catch((e) => {
        setError(String(e));
        setPhase("choose");
      });
  };

  const finish = () => {
    setPhase("choose");
    setStatus(null);
    setSelected(null);
  };

  if (phase === "deleting") {
    return (
      <div className="center">
        <div className="card panel">
          <div className="spinner" />
          <h2>{S.deleting}</h2>
          <p className="muted">{status && status.total > 0 ? S.cleaningProgress(status.done, status.total) : " "}</p>
          <div className="scan-path" title={status?.current ?? undefined}>
            {status?.current ?? " "}
          </div>
        </div>
      </div>
    );
  }
  if (phase === "done" && status) {
    return (
      <div className="center">
        <div className="card panel">
          <h2>{S.freedTitle(formatBytes(status.freed))}</h2>
          <div className="result-notes">{status.failed.length > 0 && <div className="notice">{S.failedNote(status.failed.length)}</div>}</div>
          <div className="modal-actions center-actions">
            <button className="btn primary" onClick={finish} autoFocus>
              {S.done}
            </button>
          </div>
        </div>
      </div>
    );
  }

  const now = Date.now() / 1000;
  return (
    <div className="cleanup">
      <div className="sysdata-head">
        <div>
          <h3>{S.projectsTitle}</h3>
          <p className="muted small">{S.projectsBody}</p>
        </div>
      </div>
      {error && <LoadError message={error} onRetry={load} />}
      <div className="card clean-list">
        {projects === null && <div className="empty">{S.projectsNoScan}</div>}
        {projects && projects.length === 0 && <div className="empty">{S.noProjects}</div>}
        {list.map((p) => {
          const paths = p.folders.map((f) => f.path);
          const all = paths.every((x) => chosen.has(x));
          const some = !all && paths.some((x) => chosen.has(x));
          return (
            <section key={p.path} className="project">
              <label className="clean-row project-row">
                <input
                  type="checkbox"
                  checked={all}
                  ref={(el) => {
                    if (el) el.indeterminate = some;
                  }}
                  onChange={() => toggle(paths, !all)}
                />
                <div className="clean-text">
                  <div className="sys-title">
                    <span className="name">{p.name}</span>
                    <span className={`verdict ${idleDays(p, now) >= 30 ? "safeToClear" : "managedByWindows"}`}>{S.idle(idleDays(p, now))}</span>
                  </div>
                  <div className="confirm-path" title={p.path}>
                    {p.path}
                  </div>
                </div>
                <Size bytes={p.total} />
              </label>
              {p.folders.map((f) => (
                <label key={f.path} className="clean-row folder-row">
                  <input type="checkbox" checked={chosen.has(f.path)} onChange={() => toggle([f.path], !chosen.has(f.path))} />
                  <div className="clean-text">
                    <span className="small">
                      <span className="badge">{S.junkKinds[f.kind]}</span> {f.path.slice(p.path.length + 1)}
                    </span>
                  </div>
                  <span className="muted small">{formatBytes(f.alloc)}</span>
                </label>
              ))}
            </section>
          );
        })}
      </div>
      <div className="clean-bar">
        <span className="muted">{chosenFolders.length > 0 ? S.selectedCount(chosenFolders.length) : S.nothingSelected}</span>
        <div className="spacer" />
        <button className="btn primary" disabled={chosenFolders.length === 0} onClick={() => setPhase("confirm")}>
          {S.deleteButton(formatBytes(total))}
        </button>
      </div>
      {phase === "confirm" && (
        <div className="modal-backdrop" onMouseDown={() => setPhase("choose")}>
          <div className="card modal" role="dialog" aria-modal="true" onMouseDown={(e) => e.stopPropagation()}>
            <h3>{S.deleteTitle}</h3>
            <p className="muted small">{S.deleteBody}</p>
            <div className="confirm-list">
              {chosenFolders.map((f) => (
                <div key={f.path} className="confirm-head">
                  <span className="confirm-path" title={f.path}>
                    {f.path}
                  </span>
                  <span className="size">{formatBytes(f.alloc)}</span>
                </div>
              ))}
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={() => setPhase("choose")}>
                {S.cancel}
              </button>
              <button className="btn primary danger" onClick={start}>
                {S.deleteConfirm(formatBytes(total))}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Size({ bytes }: { bytes: number }) {
  const { value, unit } = splitBytes(bytes);
  return (
    <div className="size">
      {value}
      <span className="unit">{unit}</span>
    </div>
  );
}
