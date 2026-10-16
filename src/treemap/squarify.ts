export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Tile<T> extends Rect {
  data: T;
}

interface Area<T> {
  area: number;
  data: T;
}

/**
 * Squarified treemap (Bruls, Huizing & van Wijk). Items should be sorted by
 * value, largest first. Items with no positive value are dropped.
 */
export function squarify<T>(items: { value: number; data: T }[], bounds: Rect): Tile<T>[] {
  const positive = items.filter((i) => i.value > 0);
  const total = positive.reduce((s, i) => s + i.value, 0);
  if (total === 0 || bounds.w <= 0 || bounds.h <= 0) return [];

  const scale = (bounds.w * bounds.h) / total;
  const areas: Area<T>[] = positive.map((i) => ({ area: i.value * scale, data: i.data }));
  const tiles: Tile<T>[] = [];
  let rect = { ...bounds };
  let row: Area<T>[] = [];
  let i = 0;
  while (i < areas.length) {
    const side = Math.min(rect.w, rect.h);
    const candidate = [...row, areas[i]];
    if (row.length === 0 || worst(candidate, side) <= worst(row, side)) {
      row = candidate;
      i++;
    } else {
      rect = layoutRow(row, rect, tiles);
      row = [];
    }
  }
  if (row.length > 0) layoutRow(row, rect, tiles);
  return tiles;
}

/** Worst aspect ratio in a row laid along a side of length `side`. */
function worst<T>(row: Area<T>[], side: number): number {
  const sum = row.reduce((s, r) => s + r.area, 0);
  const max = Math.max(...row.map((r) => r.area));
  const min = Math.min(...row.map((r) => r.area));
  const s2 = side * side;
  const sum2 = sum * sum;
  return Math.max((s2 * max) / sum2, sum2 / (s2 * min));
}

/** Places a row along the shorter side of `rect` and returns the space left. */
function layoutRow<T>(row: Area<T>[], rect: Rect, out: Tile<T>[]): Rect {
  const sum = row.reduce((s, r) => s + r.area, 0);
  if (rect.w >= rect.h) {
    const colW = sum / rect.h;
    let y = rect.y;
    for (const r of row) {
      const h = r.area / colW;
      out.push({ x: rect.x, y, w: colW, h, data: r.data });
      y += h;
    }
    return { x: rect.x + colW, y: rect.y, w: rect.w - colW, h: rect.h };
  }
  const rowH = sum / rect.w;
  let x = rect.x;
  for (const r of row) {
    const w = r.area / rowH;
    out.push({ x, y: rect.y, w, h: rowH, data: r.data });
    x += w;
  }
  return { x: rect.x, y: rect.y + rowH, w: rect.w, h: rect.h - rowH };
}
