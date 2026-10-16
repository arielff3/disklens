import type { SystemGroup, SystemItem } from "./types";

export const GROUP_ORDER: SystemGroup[] = ["windows", "developer", "appCaches", "tempLogs"];

export interface GroupView {
  group: SystemGroup;
  items: SystemItem[];
  total: number;
  /** Bytes in rows marked safe to clear. */
  safe: number;
}

/** Rows worth showing: still measuring, needing admin, or actually using space. */
export function isVisible(item: SystemItem): boolean {
  return item.state === "pending" || item.state === "needsAdmin" || (item.state !== "missing" && item.alloc > 0);
}

export function isMeasuring(items: SystemItem[] | null): boolean {
  return items === null || items.some((i) => i.state === "pending");
}

/** Groups in a fixed order, rows largest first; groups with nothing to show are dropped. */
export function groupItems(items: SystemItem[]): GroupView[] {
  return GROUP_ORDER.map((group) => {
    const rows = items.filter((i) => i.group === group && isVisible(i)).sort((a, b) => b.alloc - a.alloc);
    return {
      group,
      items: rows,
      total: rows.reduce((sum, i) => sum + i.alloc, 0),
      safe: rows.filter((i) => i.verdict === "safeToClear").reduce((sum, i) => sum + i.alloc, 0),
    };
  }).filter((g) => g.items.length > 0);
}
