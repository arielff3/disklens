import type { MouseEvent } from "react";
import { formatAge } from "../format";
import { FileIcon, FolderIcon } from "../icons";
import { S } from "../strings";
import { CATEGORY_COLORS } from "../theme";
import type { Hit } from "../types";
import { SizeText } from "./SizeText";

interface Props {
  hits: Hit[];
  onOpen: (hit: Hit) => void;
  onOpenFile: (hit: Hit) => void;
  onMenu: (e: MouseEvent, hit: Hit) => void;
}

/** Rows shared by Search and Biggest: badge, name, folder, age, size. */
export function ResultList({ hits, onOpen, onOpenFile, onMenu }: Props) {
  const now = Date.now() / 1000;
  const largest = hits[0]?.alloc || 1;
  return (
    <div className="card list results">
      {hits.map((h) => (
        <div
          key={h.id}
          className="row result"
          onClick={() => onOpen(h)}
          onDoubleClick={() => !h.isDir && onOpenFile(h)}
          onContextMenu={(e) => onMenu(e, h)}
        >
          <div className="item-icon">{h.isDir ? <FolderIcon /> : <FileIcon category={h.category} />}</div>
          <div className="row-name">
            <div className="name" title={h.name}>
              {h.name}
            </div>
            <div className="muted small path" title={h.parent}>
              {h.parent}
            </div>
          </div>
          <span className="badge" style={{ borderColor: h.isDir ? undefined : CATEGORY_COLORS[h.category] }}>
            {h.isDir ? S.folder : S.categories[h.category]}
          </span>
          <div className="muted small age">{formatAge(h.modified, now)}</div>
          <div className="row-bar">
            <span style={{ width: `${(h.alloc / largest) * 100}%` }} />
          </div>
          <SizeText bytes={h.alloc} />
        </div>
      ))}
      {hits.length === 0 && <div className="empty">{S.noResults}</div>}
    </div>
  );
}
