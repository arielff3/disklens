import { describe, expect, it } from "vitest";
import { defaultSelection, groupCleanItems, isSelectable, selectedTotal } from "./cleanup";
import type { CleanItem } from "./types";

const item = (id: string, over: Partial<CleanItem>): CleanItem => ({
  id,
  group: "appCaches",
  state: "ok",
  alloc: 100,
  files: 1,
  defaultOn: true,
  appOpen: null,
  roots: [],
  ...over,
});

const items = [
  item("chrome", { alloc: 300 }),
  item("spotify", { appOpen: "Spotify" }),
  item("npm", { group: "developer", defaultOn: false, alloc: 500 }),
  item("winTemp", { group: "tempLogs", state: "needsAdmin", alloc: 0 }),
  item("temp", { group: "tempLogs", state: "partial", alloc: 50 }),
  item("wait", { state: "pending", alloc: 0 }),
];

describe("cleanup", () => {
  it("only offers measured rows whose app is closed", () => {
    expect(items.filter(isSelectable).map((i) => i.id)).toEqual(["chrome", "npm", "temp"]);
  });

  it("checks the default rows that can be cleaned", () => {
    expect([...defaultSelection(items)].sort()).toEqual(["chrome", "temp"]);
  });

  it("totals only selected rows that can be cleaned", () => {
    expect(selectedTotal(items, new Set(["chrome", "spotify", "npm"]))).toBe(800);
  });

  it("groups rows with checked-by-default groups first, largest first", () => {
    const groups = groupCleanItems(items);
    expect(groups.map((g) => g.group)).toEqual(["appCaches", "tempLogs", "developer"]);
    expect(groups[0].items.map((i) => i.id)).toEqual(["chrome", "spotify", "wait"]);
  });
});
