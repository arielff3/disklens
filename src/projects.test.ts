import { describe, expect, it } from "vitest";
import { defaultSelection, idleDays, selectedTotal } from "./projects";
import type { Project } from "./types";

const DAY = 86_400;
const NOW = 1_000 * DAY;

const project = (name: string, idle: number, sizes: number[]): Project => ({
  path: `C:\\work\\${name}`,
  name,
  lastActive: NOW - idle * DAY,
  folders: sizes.map((alloc, i) => ({ path: `C:\\work\\${name}\\f${i}`, kind: "node", alloc, files: 1 })),
  total: sizes.reduce((a, b) => a + b, 0),
});

describe("projects", () => {
  const projects = [project("old", 400, [100, 50]), project("recent", 3, [500]), project("month", 30, [10])];

  it("counts idle days", () => {
    expect(idleDays(projects[0], NOW)).toBe(400);
  });

  it("checks projects untouched for 30 days or more", () => {
    expect([...defaultSelection(projects, NOW)].sort()).toEqual(["C:\\work\\month\\f0", "C:\\work\\old\\f0", "C:\\work\\old\\f1"]);
  });

  it("totals the checked folders", () => {
    expect(selectedTotal(projects, new Set(["C:\\work\\old\\f1", "C:\\work\\recent\\f0"]))).toBe(550);
  });
});
