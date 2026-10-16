import { MODES, THEMES, type Mode, type ThemeId } from "./themes";
import { themeTokens } from "./tokens";

export interface ThemeSettings {
  theme: ThemeId;
  mode: Mode;
}

export const DEFAULT_SETTINGS: ThemeSettings = { theme: "ink", mode: "dark" };

const KEY = "disklens.theme";

type Storage = Pick<globalThis.Storage, "getItem" | "setItem">;

/** `localStorage`, or null where it is unavailable. */
export function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** The saved choice, or the default when nothing valid is saved. */
export function loadSettings(storage: Storage | null = browserStorage()): ThemeSettings {
  try {
    // `?theme=&mode=` only matters in the browser preview (README screenshots).
    const query = typeof window === "undefined" ? null : new URLSearchParams(window.location.search);
    const theme = query?.get("theme");
    const mode = query?.get("mode");
    if (theme && theme in THEMES && MODES.includes(mode as Mode)) return { theme: theme as ThemeId, mode: mode as Mode };
    const saved = JSON.parse(storage?.getItem(KEY) ?? "null") as Partial<ThemeSettings> | null;
    if (saved && saved.theme && saved.theme in THEMES && MODES.includes(saved.mode as Mode)) {
      return { theme: saved.theme, mode: saved.mode as Mode };
    }
  } catch {
    // Broken JSON or blocked storage: use the default.
  }
  return DEFAULT_SETTINGS;
}

export function saveSettings(settings: ThemeSettings, storage: Storage | null = browserStorage()): void {
  try {
    storage?.setItem(KEY, JSON.stringify(settings));
  } catch {
    // Storage full or blocked: the choice just won't be remembered.
  }
}

/** Sets every theme variable on the document root. */
export function applyTheme(settings: ThemeSettings, root: HTMLElement = document.documentElement): void {
  for (const [name, value] of Object.entries(themeTokens(THEMES[settings.theme], settings.mode))) {
    root.style.setProperty(name, value);
  }
  root.style.colorScheme = settings.mode;
  root.dataset.theme = settings.theme;
  root.dataset.mode = settings.mode;
}
