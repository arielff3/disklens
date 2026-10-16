import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { defaultSelection, groupCleanItems, isSelectable, selectedTotal } from "../cleanup";
import { formatBytes, splitBytes } from "../format";
import { S } from "../strings";
import type { CleanItem, CleanupStatus } from "../types";
import { useSessionState } from "../sessionState";
import { LoadError } from "./LoadError";

type Phase = "choose" | "confirm" | "running" | "done";

const POLL_MS = 400;

interface Props {
  /** Called after a clean-up finishes, so System Data measures again. */
  onCleaned: () => void;
}

export function CleanupView({ onCleaned }: Props) {
  const [items, setItems] = useState<CleanItem[] | null>(null);
  const [selected, setSelected] = useSessionState<Set<string> | null>("cleanup.selected", null);
  const [phase, setPhase] = useSessionState<Phase>("cleanup.phase", "choose");
  const [status, setStatus] = useSessionState<CleanupStatus | null>("cleanup.status", null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const loadPlan = useCallback(() => {
    api
      .cleanupPlan()
      .then((plan) => {
        setItems(plan);
        setLoadError(null);
      })
      .catch((e) => setLoadError(String(e)));
  }, []);

  // A clean-up may still be running from an earlier visit to the tab.
  useEffect(() => {
    api
      .cleanupStatus()
      .then((s) => {
        if (s.running) {
          setStatus(s);
          setPhase("running");
        }
      })
      .catch(console.error);
    loadPlan();
  }, [loadPlan]);

  // Poll the plan while System Data is still measuring; then check the defaults once.
  const measuring = !loadError && (items === null || items.some((i) => i.state === "pending"));
  useEffect(() => {
    if (phase !== "choose") return;
    if (!measuring) {
      if (items && selected === null) setSelected(defaultSelection(items));
      return;
    }
    const timer = window.setInterval(loadPlan, POLL_MS);
    return () => window.clearInterval(timer);
  }, [phase, measuring, items, selected, loadPlan]);

  useEffect(() => {
    if (phase !== "running") return;
    const poll = () =>
      api
        .cleanupStatus()
        .then((s) => {
          setStatus(s);
          if (s.finished) {
            setPhase("done");
            onCleaned();
          }
        })
        .catch(console.error);
    poll(); // right away: the job may have finished while another tab was open
    const timer = window.setInterval(poll, POLL_MS);
    return () => window.clearInterval(timer);
  }, [phase, onCleaned]);

  const chosen = (items ?? []).filter((i) => selected?.has(i.id) && isSelectable(i));
  const total = selectedTotal(items ?? [], selected ?? new Set());

  const toggle = (id: string) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const start = () => {
    setStatus(null);
    // Switch to progress only once the job is registered, so the first poll can't see the previous one.
    api
      .cleanupStart(chosen.map((i) => i.id))
      .then(() => setPhase("running"))
      .catch((e) => {
        console.error(e);
        setPhase("choose");
      });
  };

  const finish = () => {
    setPhase("choose");
    setStatus(null);
    setItems(null);
    setSelected(null);
    loadPlan();
  };

  if (phase === "running") return <Progress status={status} />;
  if (phase === "done" && status) return <Result status={status} onDone={finish} />;

  return (
    <div className="cleanup">
      <div className="sysdata-head">
        <div>
          <h3>{S.cleanupTitle}</h3>
          <p className="muted small">{S.cleanupBody}</p>
        </div>
        <div className="spacer" />
        {measuring && (
          <span className="pill">
            <span className="mini-spinner" />
            {S.measuring}
          </span>
        )}
      </div>
      {loadError && <LoadError message={loadError} onRetry={loadPlan} />}
      <div className="card clean-list">
        {items !== null && items.length === 0 && <div className="empty">{S.nothingToClean}</div>}
        {groupCleanItems(items ?? []).map((g) => (
          <section key={g.group}>
            <h4 className="clean-group">{S.systemGroups[g.group]}</h4>
            {g.items.map((item) => (
              <CleanRow key={item.id} item={item} checked={selected?.has(item.id) ?? false} onToggle={() => toggle(item.id)} />
            ))}
          </section>
        ))}
      </div>
      <div className="clean-bar">
        <span className="muted">{chosen.length > 0 ? S.selectedCount(chosen.length) : S.nothingSelected}</span>
        <div className="spacer" />
        <button className="btn primary" disabled={chosen.length === 0} onClick={() => setPhase("confirm")}>
          {S.cleanButton(formatBytes(total))}
        </button>
      </div>
      {phase === "confirm" && <Confirm items={chosen} total={total} onCancel={() => setPhase("choose")} onConfirm={start} />}
    </div>
  );
}

function CleanRow({ item, checked, onToggle }: { item: CleanItem; checked: boolean; onToggle: () => void }) {
  const text = S.systemItems[item.id] ?? { title: item.id, explain: "" };
  const enabled = isSelectable(item);
  return (
    <label className={`clean-row${enabled ? "" : " disabled"}`}>
      <input type="checkbox" checked={enabled && checked} disabled={!enabled} onChange={onToggle} />
      <div className="clean-text">
        <div className="sys-title">
          <span className="name">{text.title}</span>
          {item.appOpen && <span className="verdict lookFirst">{S.appOpen(item.appOpen)}</span>}
        </div>
        <div className="sys-explain">{text.explain}</div>
      </div>
      <CleanSize item={item} />
    </label>
  );
}

function CleanSize({ item }: { item: CleanItem }) {
  if (item.state === "pending") return <div className="pending-bar" aria-label={S.measuring} />;
  if (item.state === "needsAdmin") return <span className="admin-badge">{S.needsAdmin}</span>;
  const { value, unit } = splitBytes(item.alloc);
  return (
    <div className="size">
      {value}
      <span className="unit">{unit}</span>
    </div>
  );
}

function Confirm({ items, total, onCancel, onConfirm }: { items: CleanItem[]; total: number; onCancel: () => void; onConfirm: () => void }) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);
  return (
    <div className="modal-backdrop" onMouseDown={onCancel}>
      <div className="card modal" role="dialog" aria-modal="true" aria-labelledby="confirm-title" onMouseDown={(e) => e.stopPropagation()}>
        <h3 id="confirm-title">{S.confirmTitle}</h3>
        <p className="muted small">{S.confirmBody}</p>
        <div className="confirm-list">
          {items.map((item) => (
            <div key={item.id} className="confirm-item">
              <div className="confirm-head">
                <span className="name">{S.systemItems[item.id]?.title ?? item.id}</span>
                <span className="size">{formatBytes(item.alloc)}</span>
              </div>
              {item.roots.map((root) => (
                <div key={root} className="confirm-path" title={root}>
                  {root}
                </div>
              ))}
            </div>
          ))}
        </div>
        <div className="modal-actions">
          <button className="btn" onClick={onCancel}>
            {S.cancel}
          </button>
          <button className="btn primary" onClick={onConfirm} autoFocus>
            {S.confirmButton(formatBytes(total))}
          </button>
        </div>
      </div>
    </div>
  );
}

function Progress({ status }: { status: CleanupStatus | null }) {
  const done = status?.done ?? 0;
  const total = status?.total ?? 0;
  return (
    <div className="center">
      <div className="card panel">
        <div className="spinner" />
        <h2>{S.cleaning}</h2>
        <p className="muted">{total > 0 ? S.cleaningProgress(done, total) : " "}</p>
        <div className="usage clean-progress">
          <span style={{ width: `${total > 0 ? (done / total) * 100 : 0}%` }} />
        </div>
        <div className="scan-path" title={status?.current ?? undefined}>
          {status?.current ?? " "}
        </div>
      </div>
    </div>
  );
}

function Result({ status, onDone }: { status: CleanupStatus; onDone: () => void }) {
  const apps = status.appsOpen.map((id) => S.systemItems[id]?.title ?? id).join(", ");
  return (
    <div className="center">
      <div className="card panel">
        <h2>{S.movedTitle(formatBytes(status.moved))}</h2>
        <p className="muted">{S.emptyBinHint}</p>
        <div className="result-notes">
          {status.inUse > 0 && <div className="notice">{S.inUseNote(status.inUse)}</div>}
          {status.appsOpen.length > 0 && <div className="notice">{S.appsOpenNote(apps)}</div>}
          {status.refused.length > 0 && <div className="notice">{S.refusedNote(status.refused.length)}</div>}
        </div>
        <div className="modal-actions center-actions">
          <button className="btn" onClick={() => api.openRecycleBin().catch(console.error)}>
            {S.openRecycleBin}
          </button>
          <button className="btn primary" onClick={onDone}>
            {S.done}
          </button>
        </div>
      </div>
    </div>
  );
}
