import { useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import { formatBytes } from "../format";
import { useTheme } from "../themes/ThemeProvider";
import { drawTreemap, type TileData } from "../treemap/draw";
import { squarify } from "../treemap/squarify";
import type { ChildView, NodeView } from "../types";

interface Props {
  node: NodeView;
  onOpenDir: (id: number) => void;
  onOpenFile: (name: string) => void;
  onMenu: (e: MouseEvent, child: ChildView) => void;
}

export function Treemap({ node, onOpenDir, onOpenFile, onMenu }: Props) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [hover, setHover] = useState<number | null>(null);

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) =>
      setSize({ w: Math.floor(entry.contentRect.width), h: Math.floor(entry.contentRect.height) }),
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const tiles = useMemo(
    () =>
      squarify<TileData>(
        node.children.map((child, i) => ({ value: child.alloc, data: { child, tile: i } })),
        { x: 0, y: 0, w: size.w, h: size.h },
      ),
    [node, size],
  );

  // Redraw when the theme changes too: the canvas reads its colors at draw time.
  const { settings } = useTheme();
  useEffect(() => {
    if (canvasRef.current) drawTreemap(canvasRef.current, tiles, size, hover);
  }, [tiles, size, hover, settings]);

  const hit = (e: MouseEvent<HTMLCanvasElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - r.left;
    const y = e.clientY - r.top;
    return tiles.find((t) => x >= t.x && x < t.x + t.w && y >= t.y && y < t.y + t.h) ?? null;
  };
  const hovered = tiles.find((t) => t.data.child.id === hover)?.data.child;

  return (
    <div ref={wrapRef} className="treemap-wrap">
      <canvas
        ref={canvasRef}
        style={{ cursor: hovered?.isDir ? "pointer" : "default" }}
        title={hovered ? `${hovered.name} · ${formatBytes(hovered.alloc)}` : ""}
        onMouseMove={(e) => setHover(hit(e)?.data.child.id ?? null)}
        onMouseLeave={() => setHover(null)}
        onClick={(e) => {
          const child = hit(e)?.data.child;
          if (child?.isDir) onOpenDir(child.id);
        }}
        onDoubleClick={(e) => {
          const child = hit(e)?.data.child;
          if (child && !child.isDir) onOpenFile(child.name);
        }}
        onContextMenu={(e) => {
          const child = hit(e)?.data.child;
          if (child) onMenu(e, child);
        }}
      />
    </div>
  );
}
