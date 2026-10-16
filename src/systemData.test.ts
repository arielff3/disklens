import { describe, expect, it } from "vitest";
import { groupItems, isMeasuring } from "./systemData";
import type { SystemItem } from "./types";

const item = (id: string, over: Partial<SystemItem>): SystemItem => ({
  id,
  group: "windows",
  verdict: "safeToClear",
  state: "ok",
  alloc: 0,
  files: 0,
  path: null,
  isDir: true,
  note: null,
  ...over,
});

describe("groupItems", () => {
  it("orders groups, hides empty rows and sorts by size", () => {
    const groups = groupItems([
      item("a", { group: "tempLogs", alloc: 10 }),
      item("b", { alloc: 5 }),
      item("c", { alloc: 50, verdict: "lookFirst" }),
      item("gone", { state: "missing" }),
      item("empty", { alloc: 0 }),
      item("admin", { state: "needsAdmin" }),
      item("wait", { group: "developer", state: "pending" }),
    ]);
    expect(groups.map((g) => g.group)).toEqual(["windows", "developer", "tempLogs"]);
    expect(groups[0].items.map((i) => i.id)).toEqual(["c", "b", "admin"]);
    expect(groups[0].total).toBe(55);
    expect(groups[0].safe).toBe(5);
  });

  it("knows when something is still measuring", () => {
    expect(isMeasuring(null)).toBe(true);
    expect(isMeasuring([item("a", { state: "pending" })])).toBe(true);
    expect(isMeasuring([item("a", {})])).toBe(false);
  });
});
