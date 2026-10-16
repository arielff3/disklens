import { useEffect, useState, type ReactNode } from "react";
import { api } from "../api";
import { formatDelta, formatWhen, relativeTo } from "../changes";
import { formatBytes } from "../format";
import { S } from "../strings";
import type { HistoryDiff, HistoryList } from "../types";
import { LoadError } from "./LoadError";

interface Props {
  /** Bumped when the index changes, so the comparison is redone. */
  version: number;
  onOpen: (path: string) => void;
}

/** What grew or shrank since an earlier scan. */
export function ChangesView({ version, onOpen }: Props) {
  const [list, setList] = useState<HistoryList | null>(null);
  const [baseline, setBaseline] = useState<number | null>(null);
  const [diff, setDiff] = useState<HistoryDiff | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .historyList()
      .then((l) => {
        setList(l);
        setError(null);
        setBaseline((b) => (b !== null && l.points.some((p) => p.scannedAt === b) ? b : (l.points[0]?.scannedAt ?? null)));
      })
      .catch((e) => setError(String(e)));
  }, [version]);

  useEffect(() => {
    if (baseline === null) return;
    api
      .historyDiff(baseline)
      .then(setDiff)
      .catch((e) => setError(String(e)));
  }, [baseline, version]);

  const head = (
    <div className="sysdata-head">
      <div>
        <h3>{S.changesTitle}</h3>
        <p className="muted small">{S.changesBody}</p>
      </div>
      <div className="spacer" />
      {list && list.points.length > 0 && (
        <label className="since">
          <span className="muted small">{S.since}</span>
          <select value={baseline ?? ""} onChange={(e) => setBaseline(Number(e.target.value))}>
            {list.points.map((p) => (
              <option key={p.scannedAt} value={p.scannedAt}>
                {formatWhen(p.scannedAt)} · {formatBytes(p.total)}
              </option>
            ))}
          </select>
        </label>
      )}
    </div>
  );

  if (error) {
    return (
      <div className="changes">
        {head}
        <LoadError message={error} onRetry={() => setError(null)} />
      </div>
    );
  }
  if (list && !list.root) return <Empty head={head} text={S.noScan} />;
  if (list && list.points.length === 0) return <Empty head={head} text={S.noHistory} />;

  const delta = diff ? diff.afterTotal - diff.beforeTotal : 0;
  const largest = Math.max(1, ...(diff?.changes ?? []).map((c) => Math.abs(c.after - c.before)));

  return (
    <div className="changes">
      {head}
      {diff && list?.root && (
        <div className="card changes-summary">
          <span className="name">{S.sinceSummary(formatWhen(diff.beforeAt), "")}</span>
          <span className={`delta ${delta >= 0 ? "grew" : "shrank"}`}>{formatDelta(delta)}</span>
          <span className="muted small">
            {formatBytes(diff.beforeTotal)} → {formatBytes(diff.afterTotal)}
          </span>
        </div>
      )}
      <div className="card changes-list">
        {diff && diff.changes.length === 0 && <div className="empty">{S.noChanges}</div>}
        {diff?.changes.map((c) => {
          const d = c.after - c.before;
          return (
            <button key={c.path} className="change-row" onClick={() => onOpen(c.path)} title={c.path}>
              <span className="change-path">{list?.root ? relativeTo(c.path, list.root) : c.path}</span>
              <span className="muted small change-sizes">
                {formatBytes(c.before)} → {formatBytes(c.after)}
              </span>
              <span className="row-bar">
                <span className={d >= 0 ? "grew" : "shrank"} style={{ width: `${(Math.abs(d) / largest) * 100}%` }} />
              </span>
              <span className={`delta ${d >= 0 ? "grew" : "shrank"}`}>{formatDelta(d)}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function Empty({ head, text }: { head: ReactNode; text: string }) {
  return (
    <div className="changes">
      {head}
      <div className="card panel center-text">
        <p className="muted">{text}</p>
      </div>
    </div>
  );
}
