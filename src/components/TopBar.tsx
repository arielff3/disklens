import { Fragment, type ReactNode } from "react";
import { BackIcon, RefreshIcon } from "../icons";
import { displayName } from "../paths";
import { S } from "../strings";
import type { Crumb } from "../types";

interface Props {
  crumbs: Crumb[];
  canGoBack: boolean;
  refreshing: boolean;
  search: ReactNode;
  onBack: () => void;
  onCrumb: (id: number) => void;
  onRescan?: () => void;
}

export function TopBar({ crumbs, canGoBack, refreshing, search, onBack, onCrumb, onRescan }: Props) {
  return (
    <div className="topbar">
      <button className="icon-btn" disabled={!canGoBack} onClick={onBack} title={S.back} aria-label={S.back}>
        <BackIcon />
      </button>
      <nav className="crumbs">
        {crumbs.map((c, i) => (
          <Fragment key={c.id}>
            {i > 0 && <span className="sep">/</span>}
            <button className={`crumb${i === crumbs.length - 1 ? " current" : ""}`} onClick={() => onCrumb(c.id)}>
              {i === 0 ? displayName(c.name) : c.name}
            </button>
          </Fragment>
        ))}
      </nav>
      <div className="spacer" />
      {refreshing && (
        <span className="pill">
          <span className="mini-spinner" />
          {S.updating}
        </span>
      )}
      {search}
      {onRescan && (
        <button className="icon-btn" onClick={onRescan} title={S.rescan} aria-label={S.rescan}>
          <RefreshIcon />
        </button>
      )}
    </div>
  );
}
