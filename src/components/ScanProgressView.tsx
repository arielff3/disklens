import { formatBytes, formatCount } from "../format";
import { displayName } from "../paths";
import { S } from "../strings";
import type { ScanProgress } from "../types";

interface Props {
  root: string;
  progress: ScanProgress | null;
  onCancel: () => void;
}

export function ScanProgressView({ root, progress, onCancel }: Props) {
  return (
    <div className="center">
      <div className="card panel">
        <div className="spinner" />
        <h2>{S.scanning(displayName(root))}</h2>
        <div className="scan-stats">
          <div>
            <b>{formatCount(progress?.files ?? 0)}</b>
            <span className="muted">{S.files}</span>
          </div>
          <div>
            <b>{formatBytes(progress?.bytes ?? 0)}</b>
            <span className="muted">{S.found}</span>
          </div>
        </div>
        <div className="scan-path" title={progress?.current}>
          {progress?.current || "\u00a0"}
        </div>
        <button className="btn" onClick={onCancel}>
          {S.cancel}
        </button>
      </div>
    </div>
  );
}
