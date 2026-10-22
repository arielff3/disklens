import { describe, expect, it } from "vitest";
import { WINDOWS_KEY, filterApps, formatPercent, sortApps, stableOrder } from "./activity";
import type { ActivityApp, ActivityProcess } from "./types";

const proc = (pid: number, name: string, cpu = 0): ActivityProcess => ({
  pid,
  parent: null,
  name,
  exe: null,
  cpu,
  ram: 0,
  gpu: null,
  gpuMem: null,
});

const app = (key: string, name: string, cpu: number, ram: number, gpu: number | null, names: string[] = []): ActivityApp => ({
  key,
  name,
  exe: null,
  processes: names.map((n, i) => proc(i + 1, n)),
  cpu,
  ram,
  gpu,
  gpuMem: null,
});

describe("activity", () => {
  const chrome = app("c:\\chrome.exe", "Google Chrome", 5, 300, 2, ["chrome.exe", "chrome_crashpad_helper.exe"]);
  const code = app("c:\\code.exe", "Visual Studio Code", 9, 100, null, ["Code.exe"]);
  const windows = app(WINDOWS_KEY, "", 1, 900, 0, ["svchost.exe"]);
  const apps = [chrome, code, windows];

  it("filters apps by app or process name", () => {
    expect(filterApps(apps, "chrome").map((a) => a.key)).toEqual([chrome.key]);
    expect(filterApps(apps, "crashpad").map((a) => a.key)).toEqual([chrome.key]);
    expect(filterApps(apps, "windows").map((a) => a.key)).toEqual([WINDOWS_KEY]);
    expect(filterApps(apps, "  ")).toBe(apps);
  });

  it("sorts by the chosen column, highest first", () => {
    expect(sortApps(apps, "cpu").map((a) => a.key)).toEqual([code.key, chrome.key, WINDOWS_KEY]);
    expect(sortApps(apps, "ram").map((a) => a.key)).toEqual([WINDOWS_KEY, chrome.key, code.key]);
    // Unknown GPU sorts after known values.
    expect(sortApps(apps, "gpu").map((a) => a.key)).toEqual([chrome.key, WINDOWS_KEY, code.key]);
  });

  it("keeps the previous order until a re-sort", () => {
    const fresh = app("c:\\new.exe", "New", 99, 0, null);
    const order = stableOrder([fresh, code, chrome], [chrome.key, WINDOWS_KEY, code.key]);
    expect(order.map((a) => a.key)).toEqual([chrome.key, code.key, fresh.key]);
  });

  it("formats percents", () => {
    expect(formatPercent(0)).toBe("0%");
    expect(formatPercent(0.04)).toBe("0%");
    expect(formatPercent(0.4)).toBe("0.4%");
    expect(formatPercent(12.34)).toBe("12%");
    expect(formatPercent(null)).toBe("—");
  });
});
