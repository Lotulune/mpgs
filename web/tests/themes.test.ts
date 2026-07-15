import { describe, expect, it } from "vitest";
import { THEME_ORDER, THEMES } from "../src/theme/registry";
import type { ThemeId } from "../src/theme/types";

describe("theme registry", () => {
  it("exposes exactly the five expected themes in order", () => {
    expect(THEME_ORDER).toEqual(["retro", "minimal", "mc", "steam", "wafu"]);
    expect(Object.keys(THEMES).sort()).toEqual(
      ["mc", "minimal", "retro", "steam", "wafu"].sort(),
    );
  });

  it("every theme provides required metadata, palette and click/action FX", () => {
    for (const id of THEME_ORDER) {
      const theme = THEMES[id];
      expect(theme.id).toBe(id);
      expect(theme.label.length).toBeGreaterThan(0);
      expect(theme.tagline.length).toBeGreaterThan(0);
      // Palette colors are concrete strings.
      expect(theme.palette.accent).toMatch(/^#|rgb/);
      expect(theme.palette.accent2).toMatch(/^#|rgb/);
      expect(theme.palette.ink).toMatch(/^#|rgb/);
      // Interactive feedback is mandatory per theme.
      expect(typeof theme.fx.click).toBe("function");
      expect(typeof theme.fx.action).toBe("function");
    }
  });

  it("theme ids are unique", () => {
    const ids = new Set<ThemeId>(THEME_ORDER);
    expect(ids.size).toBe(THEME_ORDER.length);
  });

  it("click and action handlers emit through the pool without throwing", () => {
    const emitted: string[] = [];
    const pool = {
      emit: (spec: { shape: string }) => emitted.push(spec.shape),
      liveCount: () => 0,
      liveCountOf: () => 0,
    };
    for (const id of THEME_ORDER) {
      const theme = THEMES[id];
      const before = emitted.length;
      theme.fx.click?.(pool, 100, 100, theme.palette);
      expect(emitted.length).toBeGreaterThan(before);
      for (const kind of ["like", "dismiss", "confirm", "error"] as const) {
        theme.fx.action?.(kind, pool, 100, 100, theme.palette);
      }
    }
    expect(emitted.length).toBeGreaterThan(0);
  });
});
