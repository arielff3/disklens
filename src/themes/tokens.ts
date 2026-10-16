import type { Category, PlaceKey } from "../types";
import { alpha, mixWhite, type Shade } from "./scale";
import type { Mode, Ref, ThemeDef } from "./themes";

/** Every CSS variable the UI uses, for one theme in one mode. All plain `rgb()`. */
export function themeTokens(theme: ThemeDef, mode: Mode): Record<string, string> {
  const pick = (family: string, shade: Shade) => theme.palette[family][shade];
  const ref = ([family, shade]: Ref) => pick(family, shade);
  const n = (shade: Shade) => pick(theme.neutral, shade);
  const accent = (shade: Shade) => pick(theme.accent, shade);
  const safe = (shade: Shade) => pick(theme.safe, shade);
  const caution = (shade: Shade) => pick(theme.caution, shade);

  const base =
    mode === "dark"
      ? {
          "--bg": n("950"),
          "--sidebar": pick(theme.sidebar, "950"),
          "--card": n("900"),
          "--border": alpha(n("700"), 0.45),
          "--text": n("50"),
          "--muted": alpha(pick(theme.muted, "200"), 0.7),
          "--accent": accent("500"),
          "--accent-strong": accent("400"),
          "--accent-soft": alpha(accent("500"), 0.16),
          "--accent-text": accent("200"),
          "--on-accent": n("950"),
          "--hover": alpha(n("700"), 0.35),
          "--bar": alpha(n("700"), 0.45),
          "--bar-fill": pick(theme.barFill, "600"),
          "--safe": safe("300"),
          "--safe-soft": alpha(safe("500"), 0.18),
          "--caution": caution("300"),
          "--caution-soft": alpha(caution("500"), 0.16),
          "--managed": n("200"),
          "--managed-soft": alpha(n("500"), 0.14),
          "--shadow": "0 8px 24px rgb(0 0 0 / 0.45)",
          "--tile-label": n("50"),
          "--tile-size": alpha(n("50"), 0.7),
          "--tile-hover-fill": alpha(n("50"), 0.1),
          "--tile-hover-stroke": n("100"),
          "--folder-back": n("600"),
          "--folder-front": n("500"),
          "--file-page": n("800"),
          "--file-edge": n("700"),
        }
      : {
          "--bg": pick(theme.tint, "50"),
          "--sidebar": pick(theme.tint, "100"),
          "--card": mixWhite(pick(theme.tint, "50"), 0.65),
          "--border": alpha(n("400"), 0.35),
          "--text": n("900"),
          "--muted": alpha(n("900"), 0.62),
          "--accent": accent("500"),
          "--accent-strong": accent("600"),
          "--accent-soft": alpha(accent("500"), 0.16),
          "--accent-text": accent("800"),
          "--on-accent": n("950"),
          "--hover": alpha(n("400"), 0.18),
          "--bar": alpha(n("400"), 0.3),
          "--bar-fill": pick(theme.barFill, "500"),
          "--safe": safe("700"),
          "--safe-soft": alpha(safe("500"), 0.16),
          "--caution": caution("700"),
          "--caution-soft": alpha(caution("500"), 0.18),
          "--managed": n("700"),
          "--managed-soft": alpha(n("500"), 0.12),
          "--shadow": "0 8px 24px rgb(0 0 0 / 0.14)",
          "--tile-label": n("900"),
          "--tile-size": alpha(n("900"), 0.6),
          "--tile-hover-fill": alpha(n("900"), 0.08),
          "--tile-hover-stroke": alpha(n("900"), 0.35),
          "--folder-back": n("400"),
          "--folder-front": n("300"),
          "--file-page": pick(theme.tint, "100"),
          "--file-edge": alpha(n("400"), 0.5),
        };

  const tokens: Record<string, string> = {
    ...base,
    "--logo-back": n("900"),
    "--logo-face": n("50"),
    "--logo-slice": accent("500"),
    "--drive": ref(theme.drive),
  };
  for (const [category, r] of Object.entries(theme.categories[mode]) as [Category, Ref][]) {
    tokens[`--cat-${category}`] = ref(r);
  }
  theme.tiles[mode].forEach((r, i) => (tokens[`--tile-${i}`] = ref(r)));
  for (const [place, r] of Object.entries(theme.places) as [PlaceKey, Ref][]) {
    tokens[`--place-${place}`] = ref(r);
  }
  return tokens;
}
