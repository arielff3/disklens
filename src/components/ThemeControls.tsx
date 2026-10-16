import { useEffect, useRef, useState } from "react";
import { CheckIcon, MoonIcon, SunIcon } from "../icons";
import { S } from "../strings";
import { useTheme } from "../themes/ThemeProvider";
import { THEME_ORDER, THEMES, type Mode, type ThemeId } from "../themes/themes";
import { themeTokens } from "../themes/tokens";

/** Theme menu and light/dark toggle for the bottom of the sidebar. */
export function ThemeControls() {
  const { settings, update } = useTheme();
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!wrapRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const pick = (theme: ThemeId) => {
    update({ theme });
    setOpen(false);
  };
  const nextMode: Mode = settings.mode === "dark" ? "light" : "dark";
  const modeLabel = nextMode === "light" ? S.switchToLight : S.switchToDark;

  return (
    <div className="theme-row">
      <div className="theme-menu-wrap" ref={wrapRef}>
        <button className="theme-btn" onClick={() => setOpen((o) => !o)} aria-haspopup="menu" aria-expanded={open} title={S.theme}>
          <Swatches theme={settings.theme} mode={settings.mode} />
          <span>{S.themeNames[settings.theme]}</span>
        </button>
        {open && (
          <div className="theme-menu" role="menu">
            {THEME_ORDER.map((id) => (
              <button key={id} role="menuitemradio" aria-checked={id === settings.theme} onClick={() => pick(id)}>
                <Swatches theme={id} mode={settings.mode} />
                <span>{S.themeNames[id]}</span>
                {id === settings.theme && (
                  <span className="check">
                    <CheckIcon />
                  </span>
                )}
              </button>
            ))}
          </div>
        )}
      </div>
      <button className="icon-btn mode-btn" onClick={() => update({ mode: nextMode })} title={modeLabel} aria-label={modeLabel}>
        {nextMode === "light" ? <SunIcon /> : <MoonIcon />}
      </button>
    </div>
  );
}

/** A theme's background, accent and first treemap colors in the given mode. */
function Swatches({ theme, mode }: { theme: ThemeId; mode: Mode }) {
  const t = themeTokens(THEMES[theme], mode);
  return (
    <span className="swatches" aria-hidden>
      {[t["--bg"], t["--accent"], t["--tile-0"], t["--tile-2"]].map((color, i) => (
        <span key={i} style={{ background: color }} />
      ))}
    </span>
  );
}
