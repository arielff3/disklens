import { TILE_COUNT } from "./themes/themes";
import type { AppsList, AppView } from "./types";

export function footprint(app: AppView): number {
  return app.installSize + app.dataSize;
}

/** Apps whose name or publisher contains every word of `query`, largest first. */
export function visibleApps(apps: AppView[], query: string): AppView[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return apps
    .filter((a) => {
      const text = `${a.name} ${a.publisher ?? ""}`.toLowerCase();
      return words.every((w) => text.includes(w));
    })
    .sort((a, b) => footprint(b) - footprint(a) || a.name.localeCompare(b.name));
}

export function isMeasuringApps(list: AppsList | null): boolean {
  return list === null || list.loading || list.apps.some((a) => !a.measured);
}

/** A theme tile color index derived from the name, so each app keeps its color. */
export function avatarTile(name: string): number {
  let hash = 0;
  for (const c of name) hash = (hash * 31 + c.charCodeAt(0)) >>> 0;
  return hash % TILE_COUNT;
}
