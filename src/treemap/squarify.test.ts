import { describe, expect, it } from "vitest";
import { squarify, type Tile } from "./squarify";

const bounds = { x: 0, y: 0, w: 600, h: 400 };
const area = (t: Tile<unknown>) => t.w * t.h;
const items = (values: number[]) => values.map((value, i) => ({ value, data: i }));

describe("squarify", () => {
  it("fills the bounds with areas proportional to values", () => {
    const tiles = squarify(items([6, 6, 4, 3, 2, 2, 1]), bounds);
    expect(tiles).toHaveLength(7);
    const total = tiles.reduce((s, t) => s + area(t), 0);
    expect(total).toBeCloseTo(600 * 400, 3);
    for (const t of tiles) {
      expect(area(t)).toBeCloseTo((([6, 6, 4, 3, 2, 2, 1][t.data] as number) / 24) * 600 * 400, 3);
    }
  });

  it("keeps every tile inside the bounds", () => {
    for (const t of squarify(items([50, 20, 10, 10, 5, 3, 1, 1]), bounds)) {
      expect(t.x).toBeGreaterThanOrEqual(-1e-9);
      expect(t.y).toBeGreaterThanOrEqual(-1e-9);
      expect(t.x + t.w).toBeLessThanOrEqual(600 + 1e-6);
      expect(t.y + t.h).toBeLessThanOrEqual(400 + 1e-6);
    }
  });

  it("produces reasonably square tiles", () => {
    const tiles = squarify(items([6, 6, 4, 3, 2, 2, 1]), { x: 0, y: 0, w: 6, h: 4 });
    const worst = Math.max(...tiles.map((t) => Math.max(t.w / t.h, t.h / t.w)));
    expect(worst).toBeLessThan(3);
  });

  it("gives a single item the whole area", () => {
    expect(squarify(items([5]), bounds)).toEqual([{ x: 0, y: 0, w: 600, h: 400, data: 0 }]);
  });

  it("drops zero values and handles empty input", () => {
    expect(squarify(items([0, 0]), bounds)).toEqual([]);
    expect(squarify(items([3, 0]), bounds)).toHaveLength(1);
    expect(squarify(items([3]), { x: 0, y: 0, w: 0, h: 10 })).toEqual([]);
  });
});
