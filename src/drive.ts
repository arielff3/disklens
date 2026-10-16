import type { Drive } from "./types";

export function driveTitle(d: Drive): string {
  return `${d.label} (${d.root.slice(0, 2)})`;
}

export function usedPercent(d: Drive): number {
  return d.total > 0 ? ((d.total - d.free) / d.total) * 100 : 0;
}
