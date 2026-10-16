import type { MouseEvent } from "react";
import { formatBytes, formatFiles } from "../format";
import { FileIcon, FolderIcon } from "../icons";
import { S } from "../strings";
import type { ChildView, NodeView } from "../types";
import { SizeText } from "./SizeText";

interface Props {
  node: NodeView;
  onOpenDir: (id: number) => void;
  onOpenFile: (name: string) => void;
  onMenu: (e: MouseEvent, child: ChildView) => void;
}

export function ChildList({ node, onOpenDir, onOpenFile, onMenu }: Props) {
  const largest = node.children[0]?.alloc || 1;
  return (
    <div className="card list">
      {node.children.map((c) => (
        <div
          key={c.id}
          className={`row ${c.isDir ? "dir" : "file"}`}
          onClick={() => c.isDir && onOpenDir(c.id)}
          onDoubleClick={() => !c.isDir && onOpenFile(c.name)}
          onContextMenu={(e) => onMenu(e, c)}
        >
          <div className="item-icon">{c.isDir ? <FolderIcon /> : <FileIcon category={c.category} />}</div>
          <div className="row-name">
            <div className="name" title={c.name}>
              {c.name}
            </div>
            <div className="muted small">{c.isDir ? formatFiles(c.files) : S.categories[c.category]}</div>
          </div>
          <div className="row-bar">
            <span style={{ width: `${(c.alloc / largest) * 100}%` }} />
          </div>
          <SizeText bytes={c.alloc} />
        </div>
      ))}
      {node.rest && (
        <div className="rest">
          {S.smallerItems(node.rest.count)} · {formatBytes(node.rest.alloc)}
        </div>
      )}
      {node.children.length === 0 && <div className="empty">{S.emptyFolder}</div>}
    </div>
  );
}
