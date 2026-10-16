import type { CleanItem, SystemGroup } from "./types";

/** Measured, using space, not waiting for admin, and its app is closed. */
export function isSelectable(item: CleanItem): boolean {
  return item.appOpen === null && (item.state === "ok" || item.state === "partial") && item.alloc > 0;
}

export function defaultSelection(items: CleanItem[]): Set<string> {
  return new Set(items.filter((i) => i.defaultOn && isSelectable(i)).map((i) => i.id));
}

export function selectedTotal(items: CleanItem[], selected: Set<string>): number {
  return items.filter((i) => selected.has(i.id) && isSelectable(i)).reduce((sum, i) => sum + i.alloc, 0);
}

/** What is checked by default comes first; developer caches (unchecked) last. */
const CLEAN_ORDER: SystemGroup[] = ["appCaches", "tempLogs", "windows", "developer"];

export interface CleanGroup {
  group: SystemGroup;
  items: CleanItem[];
}

/** Groups in clean-up order, rows largest first; empty groups are dropped. */
export function groupCleanItems(items: CleanItem[]): CleanGroup[] {
  return CLEAN_ORDER.map((group) => ({
    group,
    items: items.filter((i) => i.group === group).sort((a, b) => b.alloc - a.alloc),
  })).filter((g) => g.items.length > 0);
}
