// MC 风 (blocky sandbox): procedural dirt/stone/plank tiles, chunky bevel UI,
// block-break particle physics, floating pickup-style hearts and XP orbs.
// All textures are generated at runtime — no copyrighted assets.

import { makeTexture, mulberry32 } from "../proc";
import type { ParticleEmitter, ThemeFx } from "../../fx/types";
import type { ThemeDefinition } from "../types";

// Classic block palettes (approximate hues, original procedural noise).
const DIRT = ["#79553a", "#8a6142", "#6d4c34", "#7d5b3e", "#5f4229"];
const STONE = ["#7f7f7f", "#8c8c8c", "#747474", "#838383", "#6b6b6b"];
const GRASS = ["#5d9c3f", "#6aad4a", "#548f39", "#63a344", "#4c8433"];
const PLANK = ["#9c7f4e", "#a8894f", "#8f7344", "#a2834b"];

function tileTexture(palette: string[], seed: number, px = 16, cell = 4): string {
  const rng = mulberry32(seed);
  return makeTexture(px * cell, (ctx) => {
    for (let ix = 0; ix < px; ix += 1) {
      for (let iy = 0; iy < px; iy += 1) {
        const idx = Math.floor(rng() * palette.length);
        ctx.fillStyle = palette[idx] ?? palette[0] ?? "#777";
        ctx.fillRect(ix * cell, iy * cell, cell, cell);
      }
    }
  });
}

function blockBreak(pool: ParticleEmitter, x: number, y: number, colors: string[]): void {
  const count = 16;
  for (let i = 0; i < count; i += 1) {
    const angle = Math.random() * Math.PI * 2;
    const speed = 60 + Math.random() * 190;
    pool.emit({
      x: x + (Math.random() - 0.5) * 10,
      y: y + (Math.random() - 0.5) * 10,
      vx: Math.cos(angle) * speed,
      vy: Math.sin(angle) * speed - 130,
      ay: 620, // chunky gravity
      drag: 0.4,
      ttl: 0.5 + Math.random() * 0.35,
      size: 4 + Math.floor(Math.random() * 4),
      color: colors[Math.floor(Math.random() * colors.length)] ?? "#8a6142",
      shape: "pixel",
    });
  }
}

const fx: ThemeFx = {
  drawAmbient(ctx, env) {
    // Dark dimmed backdrop like a pause menu; slow drifting square "motes".
    const { width, height, time } = env;
    for (let i = 0; i < 8; i += 1) {
      const phase = i * 1.7;
      const x = ((i * 0.131 + time * 0.008) % 1) * width;
      const y = ((Math.sin(time * 0.1 + phase) + 1) / 2) * height;
      ctx.globalAlpha = 0.05;
      ctx.fillStyle = "#ffffff";
      const s = 3 + (i % 3) * 2;
      ctx.fillRect(Math.round(x), Math.round(y), s, s);
    }
    ctx.globalAlpha = 1;
  },

  ambientSpawn() {
    // The MC mood is the static texture; particles only on interaction.
  },

  click(pool, x, y, palette) {
    void palette;
    blockBreak(pool, x, y, [...DIRT, ...STONE.slice(0, 2)]);
  },

  action(kind, pool, x, y) {
    switch (kind) {
      case "like":
        // Hearts pop up like taming feedback.
        for (let i = 0; i < 6; i += 1) {
          pool.emit({
            x: x + (Math.random() - 0.5) * 44,
            y: y - Math.random() * 8,
            vy: -70 - Math.random() * 50,
            vx: (Math.random() - 0.5) * 30,
            ttl: 0.8 + Math.random() * 0.4,
            size: 8 + Math.random() * 4,
            color: "#f43b3b",
            shape: "mc-heart",
          });
        }
        break;
      case "dismiss":
        // Smoke poof, like a despawn.
        for (let i = 0; i < 12; i += 1) {
          pool.emit({
            x: x + (Math.random() - 0.5) * 26,
            y: y + (Math.random() - 0.5) * 16,
            vx: (Math.random() - 0.5) * 60,
            vy: -30 - Math.random() * 50,
            ttl: 0.5 + Math.random() * 0.4,
            size: 5 + Math.random() * 5,
            color: ["#c9c9c9", "#a8a8a8", "#8d8d8d"][i % 3] ?? "#bbb",
            shape: "pixel",
          });
        }
        break;
      case "confirm":
        // XP orbs pulled upward.
        for (let i = 0; i < 10; i += 1) {
          pool.emit({
            x: x + (Math.random() - 0.5) * 50,
            y: y + (Math.random() - 0.5) * 20,
            vy: -90 - Math.random() * 70,
            vx: (Math.random() - 0.5) * 40,
            drag: 1.2,
            ttl: 0.7 + Math.random() * 0.3,
            size: 4 + Math.random() * 3,
            color: i % 2 === 0 ? "#7dfc4a" : "#e8ff5c",
            shape: "dot",
          });
        }
        break;
      case "error":
        blockBreak(pool, x, y, ["#8b2f2f", "#a03a3a", "#6f2424"]);
        break;
    }
  },

  shapes: {
    // Pixelated heart from a 7x6 bitmap.
    "mc-heart": (ctx, p, t) => {
      const rows = [0b0110110, 0b1111111, 0b1111111, 0b0111110, 0b0011100, 0b0001000];
      const cell = Math.max(1.5, p.size / 4);
      ctx.globalAlpha = t < 0.7 ? 1 : Math.max(0, 1 - (t - 0.7) / 0.3);
      ctx.fillStyle = p.color;
      const originX = Math.round(p.x - (7 * cell) / 2);
      const originY = Math.round(p.y - (6 * cell) / 2);
      for (let r = 0; r < rows.length; r += 1) {
        const bits = rows[r] ?? 0;
        for (let c = 0; c < 7; c += 1) {
          if ((bits >> (6 - c)) & 1) {
            ctx.fillRect(originX + c * cell, originY + r * cell, cell, cell);
          }
        }
      }
    },
  },
};

export const mcTheme: ThemeDefinition = {
  id: "mc",
  label: "MC 方块",
  tagline: "像素方块与破坏粒子",
  palette: { accent: "#7dfc4a", accent2: "#f43b3b", ink: "#ffffff" },
  fx,
  onActivate(root) {
    // Generate tiling textures once and expose them to the CSS skin.
    root.style.setProperty("--mc-dirt", `url(${tileTexture(DIRT, 1)})`);
    root.style.setProperty("--mc-stone", `url(${tileTexture(STONE, 2)})`);
    root.style.setProperty("--mc-grass", `url(${tileTexture(GRASS, 3)})`);
    root.style.setProperty("--mc-plank", `url(${tileTexture(PLANK, 4)})`);
  },
};
