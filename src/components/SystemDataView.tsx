import { formatBytes, splitBytes } from "../format";
import { ExploreIcon, RefreshIcon, RevealIcon } from "../icons";
import { S } from "../strings";
import { groupItems, isMeasuring, type GroupView } from "../systemData";
import type { SystemItem } from "../types";
import { CompactDialog } from "./CompactDialog";
import { useSessionState } from "../sessionState";
import { LoadError } from "./LoadError";
import { WindowsTools } from "./WindowsTools";

/** Rows whose virtual disks can be compacted. */
const COMPACTABLE = new Set(["wslDisks", "dockerDisk"]);

interface Actions {
  onOpenInExplore: (path: string) => void;
  onReveal: (path: string) => void;
  onCompact: () => void;
}

interface Props extends Actions {
  items: SystemItem[] | null;
  /** Loading failed; shown instead of an endless spinner. */
  error: string | null;
  elevated: boolean;
  onRefresh: () => void;
  onRunAsAdmin: () => void;
}

export function SystemDataView({ items, error, elevated, onRefresh, onRunAsAdmin, ...rest }: Omit<Props, "onCompact">) {
  const [compacting, setCompacting] = useSessionState("sysdata.compacting", false);
  const actions: Actions = { ...rest, onCompact: () => setCompacting(true) };
  const measuring = !error && isMeasuring(items);
  const lacksAdmin = !elevated && (items ?? []).some((i) => i.state === "needsAdmin");
  return (
    <div className="sysdata">
      <div className="sysdata-head">
        <div>
          <h3>{S.systemDataTitle}</h3>
          <p className="muted small">{S.systemDataBody}</p>
        </div>
        <div className="spacer" />
        {measuring && (
          <span className="pill">
            <span className="mini-spinner" />
            {S.measuring}
          </span>
        )}
        <button className="icon-btn" onClick={onRefresh} disabled={measuring} title={S.measureAgain} aria-label={S.measureAgain}>
          <RefreshIcon />
        </button>
      </div>
      {lacksAdmin && (
        <div className="card sysdata-admin">
          <div className="notice">{S.adminNotice}</div>
          <button className="btn primary" onClick={onRunAsAdmin}>
            {S.runAsAdmin}
          </button>
        </div>
      )}
      {error && <LoadError message={error} onRetry={() => onRefresh()} />}
      <div className="sysdata-grid">
        {groupItems(items ?? []).map((g) => (
          <GroupCard key={g.group} group={g} {...actions} />
        ))}
        <WindowsTools onChanged={onRefresh} />
      </div>
      {compacting && (
        <CompactDialog
          onClose={(changed) => {
            setCompacting(false);
            if (changed) onRefresh();
          }}
        />
      )}
    </div>
  );
}

function GroupCard({ group, ...actions }: Actions & { group: GroupView }) {
  const { value, unit } = splitBytes(group.total);
  return (
    <section className="card group-card">
      <h4>{S.systemGroups[group.group]}</h4>
      <div className="group-total">
        {value}
        <span className="unit">{unit}</span>
      </div>
      {group.safe > 0 && <div className="group-safe">{S.safeToClearAmount(formatBytes(group.safe))}</div>}
      <div className="sys-rows">
        {group.items.map((item) => (
          <ItemRow key={item.id} item={item} {...actions} />
        ))}
      </div>
    </section>
  );
}

function ItemRow({ item, onOpenInExplore, onReveal, onCompact }: Actions & { item: SystemItem }) {
  const text = S.systemItems[item.id] ?? { title: item.id, explain: "" };
  const path = item.path;
  return (
    <div className="sys-row">
      <div className="sys-title">
        <span className="name">{text.title}</span>
        <span className={`verdict ${item.verdict}`}>{S.verdicts[item.verdict]}</span>
      </div>
      <div className="sys-size">
        {COMPACTABLE.has(item.id) && (item.state === "ok" || item.state === "partial") && (
          <button className="btn compact-btn" onClick={onCompact}>
            {S.compact}
          </button>
        )}
        {path && (
          <div className="sys-actions">
            {item.isDir && (
              <button className="icon-btn" onClick={() => onOpenInExplore(path)} title={S.openInExplore} aria-label={S.openInExplore}>
                <ExploreIcon />
              </button>
            )}
            <button className="icon-btn" onClick={() => onReveal(path)} title={S.reveal} aria-label={S.reveal}>
              <RevealIcon />
            </button>
          </div>
        )}
        <ItemSize item={item} />
      </div>
      <div className="sys-explain">{text.explain}</div>
      {item.note && <div className="sys-note">{S.systemNotes[item.note]}</div>}
    </div>
  );
}

function ItemSize({ item }: { item: SystemItem }) {
  if (item.state === "pending") return <div className="pending-bar" aria-label={S.measuring} />;
  if (item.state === "needsAdmin") return <span className="admin-badge">{S.needsAdmin}</span>;
  const { value, unit } = splitBytes(item.alloc);
  const partial = item.state === "partial";
  return (
    <div className="size" title={partial ? S.partialSize : undefined}>
      {value}
      <span className="unit">
        {unit}
        {partial ? "+" : ""}
      </span>
    </div>
  );
}
