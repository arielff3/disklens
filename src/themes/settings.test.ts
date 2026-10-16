import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS, loadSettings, saveSettings } from "./settings";

function memoryStorage(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial));
  return {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, v),
  };
}

describe("theme settings", () => {
  it("round-trips a saved choice", () => {
    const storage = memoryStorage();
    saveSettings({ theme: "lagoon", mode: "light" }, storage);
    expect(loadSettings(storage)).toEqual({ theme: "lagoon", mode: "light" });
  });

  it("falls back to the default for missing, broken or unknown values", () => {
    expect(loadSettings(memoryStorage())).toEqual(DEFAULT_SETTINGS);
    expect(loadSettings(memoryStorage({ "disklens.theme": "{oops" }))).toEqual(DEFAULT_SETTINGS);
    expect(loadSettings(memoryStorage({ "disklens.theme": '{"theme":"neon","mode":"dark"}' }))).toEqual(DEFAULT_SETTINGS);
    expect(loadSettings(null)).toEqual(DEFAULT_SETTINGS);
  });
});
