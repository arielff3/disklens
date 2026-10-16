import type { Project } from "./types";

/** Projects untouched for this long start checked. */
export const STALE_DAYS = 30;
const DAY = 86_400;

export function idleDays(project: Project, nowSeconds: number): number {
  return Math.max(0, Math.floor((nowSeconds - project.lastActive) / DAY));
}

export function defaultSelection(projects: Project[], nowSeconds: number): Set<string> {
  return new Set(projects.filter((p) => idleDays(p, nowSeconds) >= STALE_DAYS).flatMap((p) => p.folders.map((f) => f.path)));
}

export function selectedTotal(projects: Project[], selected: Set<string>): number {
  return projects.flatMap((p) => p.folders).filter((f) => selected.has(f.path)).reduce((sum, f) => sum + f.alloc, 0);
}
