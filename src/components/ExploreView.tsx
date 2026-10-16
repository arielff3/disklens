import type { MouseEvent } from "react";
import type { ChildView, NodeView } from "../types";
import { ChildList } from "./ChildList";
import { SummaryCard } from "./SummaryCard";
import { Treemap } from "./Treemap";

interface Props {
  node: NodeView;
  unreadable: number;
  onOpenDir: (id: number) => void;
  onOpenFile: (name: string) => void;
  onMenu: (e: MouseEvent, child: ChildView) => void;
}

export function ExploreView({ node, unreadable, onOpenDir, onOpenFile, onMenu }: Props) {
  const atRoot = node.crumbs.length === 1;
  return (
    <div className="explore">
      <div className="explore-left">
        <SummaryCard node={node} unreadable={atRoot ? unreadable : 0} />
        <ChildList node={node} onOpenDir={onOpenDir} onOpenFile={onOpenFile} onMenu={onMenu} />
      </div>
      <div className="card treemap-card">
        <Treemap node={node} onOpenDir={onOpenDir} onOpenFile={onOpenFile} onMenu={onMenu} />
      </div>
    </div>
  );
}
