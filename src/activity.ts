import { S } from "./strings";
import type { ActivityApp } from "./types";

/** Key of the group holding Windows' own processes (matches the backend). */
export const WINDOWS_KEY = "windows";

export type SortKey = "cpu" | "ram" | "gpu";

/** The name shown for an app; the backend leaves the Windows group unnamed. */
export function appName(app: ActivityApp): string {
  return app.key === WINDOWS_KEY ? S.windowsProcesses : app.name;
}

/** Apps whose name, or one of whose processes' names, contains `query`. */
export function filterApps(apps: ActivityApp[], query: string): ActivityApp[] {
  const q = query.trim().toLowerCase();
  if (!q) return apps;
  return apps.filter(
    (a) => appName(a).toLowerCase().includes(q) || a.processes.some((p) => p.name.toLowerCase().includes(q)),
  );
}

/** Highest first; apps without a value (no GPU counters) go last. */
export function sortApps(apps: ActivityApp[], key: SortKey): ActivityApp[] {
  return [...apps].sort((a, b) => (b[key] ?? -1) - (a[key] ?? -1) || appName(a).localeCompare(appName(b)));
}

/**
 * Apps in a previous order, so rows don't jump while you read: apps still running keep their
 * place, new ones go to the end, and ones that exited drop out.
 */
export function stableOrder(apps: ActivityApp[], previous: string[]): ActivityApp[] {
  const rank = new Map(previous.map((key, i) => [key, i]));
  return [...apps].sort((a, b) => (rank.get(a.key) ?? Infinity) - (rank.get(b.key) ?? Infinity));
}

/** "12%", "0.4%", or "—" when unknown. Values under 0.05 read as 0. */
export function formatPercent(value: number | null): string {
  if (value === null) return "—";
  if (value < 0.05) return "0%";
  if (value < 1) return `${value.toFixed(1)}%`;
  return `${Math.round(value)}%`;
}
