import { describe, expect, it } from "vitest";
import { MODES, THEMES } from "./themes";
import { themeTokens } from "./tokens";

describe("themeTokens", () => {
  const reference = Object.keys(themeTokens(THEMES.ink, "dark")).sort();

  it("gives every theme and mode the same set of plain colors", () => {
    for (const theme of Object.values(THEMES)) {
      for (const mode of MODES) {
        const tokens = themeTokens(theme, mode);
        expect(Object.keys(tokens).sort(), `${theme.id} ${mode}`).toEqual(reference);
        for (const [name, value] of Object.entries(tokens)) {
          expect(value, `${theme.id} ${mode} ${name}`).toMatch(/^(rgb\(\d+ \d+ \d+( \/ [\d.]+)?\)|0 8px .*)$/);
        }
      }
    }
  });

  it("keeps Ink dark exactly as it was", () => {
    const t = themeTokens(THEMES.ink, "dark");
    expect(t["--bg"]).toBe("rgb(0 26 36)");
    expect(t["--sidebar"]).toBe("rgb(0 30 36)");
    expect(t["--card"]).toBe("rgb(0 37 51)");
    expect(t["--muted"]).toBe("rgb(183 225 211 / 0.7)");
    expect(t["--accent"]).toBe("rgb(255 166 0)");
    expect(t["--tile-0"]).toBe("rgb(0 110 153)");
  });

  it("brings back the original Sand look in light mode", () => {
    const t = themeTokens(THEMES.sand, "light");
    expect(t["--bg"]).toBe("rgb(247 242 235)");
    expect(t["--sidebar"]).toBe("rgb(241 234 225)");
    expect(t["--text"]).toBe("rgb(43 37 34)");
    expect(t["--accent"]).toBe("rgb(242 107 42)");
  });
});
