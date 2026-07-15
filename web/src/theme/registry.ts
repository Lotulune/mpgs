// Theme registry + activation. The active theme is applied by setting
// [data-theme] on <html>, loading its FX module into the engine, and running
// its onActivate hook (procedural texture variables).

import { fxEngine } from "../fx/engine";
import { mcTheme } from "./themes/mc";
import { minimalTheme } from "./themes/minimal";
import { retroTheme } from "./themes/retro";
import { steamTheme } from "./themes/steam";
import { wafuTheme } from "./themes/wafu";
import type { ThemeDefinition, ThemeId } from "./types";

export const THEMES: Record<ThemeId, ThemeDefinition> = {
  retro: retroTheme,
  minimal: minimalTheme,
  mc: mcTheme,
  steam: steamTheme,
  wafu: wafuTheme,
};

export const THEME_ORDER: ThemeId[] = ["retro", "minimal", "mc", "steam", "wafu"];

const THEME_KEY = "mpgs.theme.v1";

export function isThemeId(value: string | null): value is ThemeId {
  return value !== null && value in THEMES;
}

export function loadSavedTheme(storage: Storage = globalThis.localStorage): ThemeId | null {
  try {
    const saved = storage.getItem(THEME_KEY);
    return isThemeId(saved) ? saved : null;
  } catch {
    return null;
  }
}

export function saveTheme(id: ThemeId, storage: Storage = globalThis.localStorage): void {
  try {
    storage.setItem(THEME_KEY, id);
  } catch {
    // Preference persistence is best-effort.
  }
}

/** Apply a theme to the document and the FX engine. */
export function activateTheme(id: ThemeId): ThemeDefinition {
  const theme = THEMES[id];
  const root = document.documentElement;
  root.dataset.theme = id;
  theme.onActivate?.(root);
  fxEngine.setThemeFx(theme.fx, theme.palette);
  return theme;
}
