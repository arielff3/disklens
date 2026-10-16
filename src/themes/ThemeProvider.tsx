import { createContext, useCallback, useContext, useLayoutEffect, useState, type ReactNode } from "react";
import { applyTheme, loadSettings, saveSettings, type ThemeSettings } from "./settings";

interface ThemeContextValue {
  settings: ThemeSettings;
  /** Changes part of the settings, based on the latest value. */
  update: (patch: Partial<ThemeSettings>) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

/** Applies the saved theme before the first paint and keeps it applied and saved. */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState(loadSettings);
  useLayoutEffect(() => {
    applyTheme(settings);
    saveSettings(settings);
  }, [settings]);
  const update = useCallback((patch: Partial<ThemeSettings>) => setSettings((s) => ({ ...s, ...patch })), []);
  return <ThemeContext.Provider value={{ settings, update }}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const value = useContext(ThemeContext);
  if (!value) throw new Error("useTheme needs a ThemeProvider");
  return value;
}
