export type Rect = { x: number; y: number; w: number; h: number };

// Worst aspect ratio of a row of areas laid along a side of the given length.
function worst(areas: number[], sum: number, side: number): number {
  const max = Math.max(...areas);
  const min = Math.min(...areas);
  const s2 = sum * sum;
  const w2 = side * side;
  return Math.max((w2 * max) / s2, s2 / (w2 * min));
}

/**
 * Squarified treemap (Bruls, Huizing, van Wijk). Values must be sorted largest
 * first and positive; returns one rectangle per value, in the same order.
 */
export function squarify(values: number[], box: Rect): Rect[] {
  const total = values.reduce((a, b) => a + b, 0);
  const out: Rect[] = new Array(values.length);
  if (total <= 0) return out.fill({ x: box.x, y: box.y, w: 0, h: 0 });
  const areas = values.map((v) => (v / total) * box.w * box.h);
  let { x, y, w, h } = box;
  let i = 0;
  while (i < areas.length) {
    const side = Math.min(w, h);
    const row = [areas[i]];
    let sum = areas[i];
    let j = i + 1;
    while (j < areas.length && worst([...row, areas[j]], sum + areas[j], side) <= worst(row, sum, side)) {
      row.push(areas[j]);
      sum += areas[j];
      j++;
    }
    const thick = sum / side;
    let offset = 0;
    for (let k = i; k < j; k++) {
      const len = areas[k] / thick;
      out[k] = w >= h ? { x, y: y + offset, w: thick, h: len } : { x: x + offset, y, w: len, h: thick };
      offset += len;
    }
    if (w >= h) {
      x += thick;
      w -= thick;
    } else {
      y += thick;
      h -= thick;
    }
    i = j;
  }
  return out;
}
