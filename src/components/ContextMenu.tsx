import { useEffect } from "react";
import { api } from "../api";
import { S } from "../strings";

export interface MenuTarget {
  x: number;
  y: number;
  id: number;
  isDir: boolean;
  path: string;
}

interface Props {
  target: MenuTarget;
  onClose: () => void;
  onOpenDir: (id: number) => void;
}

const MENU_W = 200;
const MENU_H = 120;

export function ContextMenu({ target, onClose, onOpenDir }: Props) {
  useEffect(() => {
    const close = () => onClose();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("blur", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("blur", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  const run = (action: () => unknown) => () => {
    Promise.resolve(action()).catch(console.error);
    onClose();
  };

  return (
    <div
      className="ctx"
      style={{ left: Math.min(target.x, window.innerWidth - MENU_W), top: Math.min(target.y, window.innerHeight - MENU_H) }}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <button onClick={run(() => (target.isDir ? onOpenDir(target.id) : api.openPath(target.path)))}>{S.open}</button>
      <button onClick={run(() => api.revealInExplorer(target.path))}>{S.reveal}</button>
      <button onClick={run(() => navigator.clipboard.writeText(target.path))}>{S.copyPath}</button>
    </div>
  );
}
