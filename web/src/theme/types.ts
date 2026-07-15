// Theme system contract.
//
// A theme is: (1) a CSS skin activated via [data-theme] on <html>, defined in
// styles/themes.css; (2) an FX module driving the canvas engine; (3) metadata
// for the picker. Themes must not load remote assets — every texture is
// generated procedurally at runtime.

import type { FxPalette, ThemeFx } from "../fx/types";

export type ThemeId = "retro" | "minimal" | "mc" | "steam" | "wafu";

export interface ThemeDefinition {
  id: ThemeId;
  /** Display name shown in the picker. */
  label: string;
  /** One-line flavor text for the picker card. */
  tagline: string;
  palette: FxPalette;
  fx: ThemeFx;
  /**
   * Called when the theme becomes active; may set generated CSS variables
   * (e.g. procedural texture data URLs) on the root element.
   */
  onActivate?: (root: HTMLElement) => void;
}
