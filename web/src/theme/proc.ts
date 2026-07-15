// Small deterministic PRNG + helpers for procedural texture generation.
// Aesthetic-only randomness; recommendation logic never uses this.

export function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a += 0x6d2b79f5;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function rand(min: number, max: number, rng: () => number = Math.random): number {
  return min + (max - min) * rng();
}

export function pick<T>(items: readonly T[], rng: () => number = Math.random): T {
  const idx = Math.min(items.length - 1, Math.floor(rng() * items.length));
  return items[idx] as T;
}

/** Parse `#rrggbb` to [r, g, b]. */
export function hexToRgb(hex: string): [number, number, number] {
  const value = hex.replace("#", "");
  const n = parseInt(value, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function rgba(hex: string, alpha: number): string {
  const [r, g, b] = hexToRgb(hex);
  return `rgba(${r},${g},${b},${alpha})`;
}

/** Lighten/darken a hex color by a factor in [-1, 1]. */
export function shade(hex: string, factor: number): string {
  const [r, g, b] = hexToRgb(hex);
  const adjust = (v: number) =>
    Math.max(0, Math.min(255, Math.round(factor >= 0 ? v + (255 - v) * factor : v * (1 + factor))));
  return `rgb(${adjust(r)},${adjust(g)},${adjust(b)})`;
}

/** Generate a tiling texture as a data URL via an offscreen canvas. */
export function makeTexture(
  size: number,
  draw: (ctx: CanvasRenderingContext2D, size: number) => void,
): string {
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) return "";
  draw(ctx, size);
  return canvas.toDataURL("image/png");
}
