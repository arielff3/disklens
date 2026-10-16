import { formatBytes, formatFiles, percent, splitBytes } from "../format";
import { S } from "../strings";
import { CATEGORY_COLORS } from "../theme";
import type { NodeView } from "../types";

interface Props {
  node: NodeView;
  /** Folders the scan couldn't read; shown only when non-zero. */
  unreadable: number;
}

export function SummaryCard({ node, unreadable }: Props) {
  const { value, unit } = splitBytes(node.alloc);
  const total = node.categories.reduce((s, c) => s + c.alloc, 0);
  return (
    <div className="card summary">
      <div className="summary-head">
        <div className="big">
          {value}
          <span className="unit">{unit}</span>
        </div>
        <div className="muted">{formatFiles(node.files)}</div>
      </div>
      <div className="catbar">
        {node.categories.map((c) => (
          <span
            key={c.category}
            title={`${S.categories[c.category]} · ${formatBytes(c.alloc)}`}
            style={{ width: `${percent(c.alloc, total) * 100}%`, background: CATEGORY_COLORS[c.category] }}
          />
        ))}
      </div>
      <div className="legend">
        {node.categories.map((c) => (
          <div key={c.category} className="legend-row">
            <span className="dot" style={{ background: CATEGORY_COLORS[c.category] }} />
            <span>{S.categories[c.category]}</span>
            <span className="legend-size">{formatBytes(c.alloc)}</span>
          </div>
        ))}
      </div>
      {unreadable > 0 && <div className="notice">{S.unreadable(unreadable)}</div>}
    </div>
  );
}
