import { formatBytes } from "../format";
import { TILE_COUNT } from "../themes/themes";
import type { ChildView } from "../types";
import type { Tile } from "./squarify";

export interface TileData {
  child: ChildView;
  /** Which theme tile color (`--tile-N`) fills it. */
  tile: number;
}

const GAP = 3;
const RADIUS = 8;
const PAD = 8;

export function drawTreemap(
  canvas: HTMLCanvasElement,
  tiles: Tile<TileData>[],
  size: { w: number; h: number },
  hoverId: number | null,
): void {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.max(1, Math.round(size.w * dpr));
  canvas.height = Math.max(1, Math.round(size.h * dpr));
  canvas.style.width = `${size.w}px`;
  canvas.style.height = `${size.h}px`;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, size.w, size.h);
  ctx.textBaseline = "top";

  // The canvas can't use CSS variables directly; read the theme's values once per draw.
  const css = getComputedStyle(canvas);
  const color = (name: string) => css.getPropertyValue(name).trim();
  const fills = Array.from({ length: TILE_COUNT }, (_, i) => color(`--tile-${i}`));
  const ink = {
    label: color("--tile-label"),
    size: color("--tile-size"),
    hoverFill: color("--tile-hover-fill"),
    hoverStroke: color("--tile-hover-stroke"),
  };

  for (const t of tiles) {
    const x = t.x + GAP / 2;
    const y = t.y + GAP / 2;
    const w = t.w - GAP;
    const h = t.h - GAP;
    if (w <= 0.5 || h <= 0.5) continue;

    ctx.beginPath();
    ctx.roundRect(x, y, w, h, Math.min(RADIUS, w / 2, h / 2));
    ctx.fillStyle = fills[t.data.tile % TILE_COUNT];
    ctx.fill();
    if (t.data.child.id === hoverId) {
      ctx.fillStyle = ink.hoverFill;
      ctx.fill();
      ctx.strokeStyle = ink.hoverStroke;
      ctx.lineWidth = 1.5;
      ctx.stroke();
    }

    if (w > 56 && h > 34) {
      ctx.fillStyle = ink.label;
      ctx.font = '600 12px "Segoe UI Variable Text", "Segoe UI", sans-serif';
      ctx.fillText(ellipsize(ctx, t.data.child.name, w - PAD * 2), x + PAD, y + PAD);
      ctx.fillStyle = ink.size;
      ctx.font = '10.5px "Cascadia Mono", Consolas, monospace';
      ctx.fillText(formatBytes(t.data.child.alloc), x + PAD, y + PAD + 16);
    }
  }
}

function ellipsize(ctx: CanvasRenderingContext2D, text: string, max: number): string {
  if (ctx.measureText(text).width <= max) return text;
  let lo = 0;
  let hi = text.length;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (ctx.measureText(`${text.slice(0, mid)}…`).width <= max) lo = mid;
    else hi = mid - 1;
  }
  return lo > 0 ? `${text.slice(0, lo)}…` : "";
}
