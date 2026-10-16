import { driveTitle, usedPercent } from "../drive";
import { formatBytes } from "../format";
import { DriveIcon } from "../icons";
import { S } from "../strings";
import { DRIVE_COLOR } from "../theme";
import type { Drive } from "../types";

interface Props {
  drives: Drive[];
  onScan: (root: string) => void;
}

export function Welcome({ drives, onScan }: Props) {
  return (
    <div className="center">
      <div className="card panel">
        <h2>{S.welcomeTitle}</h2>
        <p className="muted">{S.welcomeBody}</p>
        <div className="drive-cards">
          {drives.map((d) => (
            <button key={d.root} className="drive-card" onClick={() => onScan(d.root)}>
              <span className="side-icon" style={{ background: DRIVE_COLOR }}>
                <DriveIcon />
              </span>
              <div className="drive-info">
                <div className="drive-name">{driveTitle(d)}</div>
                <div className="muted small">
                  {formatBytes(d.free)} {S.freeOf} {formatBytes(d.total)}
                </div>
                <div className="usage">
                  <span style={{ width: `${usedPercent(d)}%` }} />
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
