import { describe, expect, it } from "vitest";
import { avatarTile, footprint, isMeasuringApps, visibleApps } from "./apps";
import type { AppView } from "./types";

const app = (id: string, over: Partial<AppView>): AppView => ({
  id,
  kind: "program",
  name: id,
  publisher: null,
  version: null,
  installLocation: null,
  installSize: 0,
  estimated: false,
  dataFolders: [],
  dataSize: 0,
  measured: true,
  ...over,
});

describe("apps", () => {
  it("adds install and data sizes", () => {
    expect(footprint(app("a", { installSize: 100, dataSize: 50 }))).toBe(150);
  });

  it("filters by name or publisher and sorts largest first", () => {
    const apps = [
      app("Discord", { installSize: 10, publisher: "Discord Inc." }),
      app("Google Chrome", { installSize: 500, publisher: "Google LLC" }),
      app("Spotify", { kind: "store", installSize: 300 }),
    ];
    expect(visibleApps(apps, "").map((a) => a.name)).toEqual(["Google Chrome", "Spotify", "Discord"]);
    expect(visibleApps(apps, "goo").map((a) => a.name)).toEqual(["Google Chrome"]);
    expect(visibleApps(apps, "  DISCORD inc ").map((a) => a.name)).toEqual(["Discord"]);
  });

  it("knows when the list is still filling in", () => {
    expect(isMeasuringApps(null)).toBe(true);
    expect(isMeasuringApps({ loading: true, apps: [] })).toBe(true);
    expect(isMeasuringApps({ loading: false, apps: [app("a", { measured: false })] })).toBe(true);
    expect(isMeasuringApps({ loading: false, apps: [app("a", {})] })).toBe(false);
  });

  it("gives each app a stable avatar color", () => {
    expect(avatarTile("Discord")).toBe(avatarTile("Discord"));
    expect(avatarTile("Discord")).toBeGreaterThanOrEqual(0);
    expect(avatarTile("Discord")).toBeLessThan(8);
  });
});
