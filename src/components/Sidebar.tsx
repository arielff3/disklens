import { driveTitle, usedPercent } from "../drive";
import { formatBytes } from "../format";
import { DriveIcon, Logo, PlaceIcon } from "../icons";
import { samePath } from "../paths";
import { APP_NAME, S } from "../strings";
import { DRIVE_COLOR } from "../theme";
import { ThemeControls } from "./ThemeControls";
import type { Drive, Place } from "../types";

interface Props {
  drives: Drive[];
  places: Place[];
  activePath: string | null;
  onDrive: (drive: Drive) => void;
  onPlace: (path: string) => void;
  elevated: boolean;
  onRunAsAdmin: () => void;
}

export function Sidebar({ drives, places, activePath, onDrive, onPlace, elevated, onRunAsAdmin }: Props) {
  const active = (path: string) => (activePath !== null && samePath(path, activePath) ? " active" : "");
  return (
    <aside className="sidebar">
      <div className="brand">
        <Logo />
        {APP_NAME}
      </div>
      <div className="section-title">{S.drives}</div>
      {drives.map((d) => (
        <button key={d.root} className={`side-item${active(d.root)}`} onClick={() => onDrive(d)}>
          <span className="side-icon" style={{ background: DRIVE_COLOR }}>
            <DriveIcon />
          </span>
          <div className="drive-info">
            <div className="drive-name">{driveTitle(d)}</div>
            <div className="muted small">
              {formatBytes(d.free)} {S.free}
            </div>
            <div className="usage">
              <span style={{ width: `${usedPercent(d)}%` }} />
            </div>
          </div>
        </button>
      ))}
      <div className="section-title">{S.places}</div>
      {places.map((p) => (
        <button key={p.key} className={`side-item${active(p.path)}`} onClick={() => onPlace(p.path)} title={p.path}>
          <PlaceIcon place={p.key} />
          {S.placeNames[p.key]}
        </button>
      ))}
      <div className="sidebar-foot">
        {elevated ? (
          <div className="admin-pill">
            <span className="dot" />
            {S.administrator}
          </div>
        ) : (
          <button className="admin-pill" onClick={onRunAsAdmin}>
            <span className="dot" />
            {S.runAsAdmin}
          </button>
        )}
        <ThemeControls />
      </div>
    </aside>
  );
}
