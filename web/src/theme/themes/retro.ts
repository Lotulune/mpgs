// 复古电子 (retro-electronic): CRT phosphor, neon grid horizon, glitch energy.

import { rgba } from "../proc";
import type { ParticleEmitter, ThemeFx } from "../../fx/types";
import type { ThemeDefinition } from "../types";

const CYAN = "#19e3e3";
const MAGENTA = "#ff3ec8";
const AMBER = "#ffc857";
const GRID = "#7a2bd6";

// Deterministic star field in normalized coordinates.
const STARS: { x: number; y: number; phase: number; size: number }[] = [];
{
  let seed = 20260714;
  const rng = () => {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 4294967296;
  };
  for (let i = 0; i < 90; i += 1) {
    STARS.push({ x: rng(), y: rng() * 0.5, phase: rng() * Math.PI * 2, size: 0.6 + rng() * 1.4 });
  }
}

function sparkBurst(
  pool: ParticleEmitter,
  x: number,
  y: number,
  colors: string[],
  count: number,
  speed: number,
): void {
  for (let i = 0; i < count; i += 1) {
    const angle = (Math.PI * 2 * i) / count + Math.random() * 0.6;
    const v = speed * (0.55 + Math.random() * 0.7);
    pool.emit({
      x,
      y,
      vx: Math.cos(angle) * v,
      vy: Math.sin(angle) * v,
      drag: 2.2,
      ay: 260,
      ttl: 0.4 + Math.random() * 0.3,
      size: 5 + Math.random() * 4,
      color: colors[i % colors.length] ?? CYAN,
      shape: "spark",
    });
  }
}

const fx: ThemeFx = {
  drawAmbient(ctx, env) {
    const { width, height, time } = env;
    const horizon = height * 0.66;

    // Star field with slow twinkle.
    for (const star of STARS) {
      const tw = 0.35 + 0.65 * Math.abs(Math.sin(time * 0.8 + star.phase));
      ctx.globalAlpha = tw * 0.5;
      ctx.fillStyle = star.phase > 4.5 ? MAGENTA : CYAN;
      ctx.fillRect(star.x * width, star.y * horizon, star.size, star.size);
    }
    ctx.globalAlpha = 1;

    // Horizon glow.
    const glow = ctx.createLinearGradient(0, horizon - 90, 0, horizon + 40);
    glow.addColorStop(0, "rgba(0,0,0,0)");
    glow.addColorStop(0.75, rgba(MAGENTA, 0.16));
    glow.addColorStop(1, rgba(CYAN, 0.1));
    ctx.fillStyle = glow;
    ctx.fillRect(0, horizon - 90, width, 130);

    // Perspective grid: verticals converge to the vanishing point.
    const vpx = width / 2;
    ctx.lineWidth = 1;
    const columns = 17;
    for (let i = 0; i <= columns; i += 1) {
      const n = i / columns - 0.5;
      const xBottom = vpx + n * width * 2.2;
      ctx.strokeStyle = rgba(GRID, 0.34 - Math.abs(n) * 0.2);
      ctx.beginPath();
      ctx.moveTo(vpx, horizon);
      ctx.lineTo(xBottom, height);
      ctx.stroke();
    }
    // Horizontal scan rows accelerate toward the viewer.
    const rows = 11;
    const scroll = (time * 0.55) % 1;
    for (let i = 0; i < rows; i += 1) {
      const z = ((i + scroll) / rows) ** 2.2;
      const y = horizon + z * (height - horizon);
      ctx.strokeStyle = rgba(GRID, 0.14 + z * 0.4);
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    // Rare glitch slab: a thin displaced bar of chroma.
    const g = (time * 0.5) % 1;
    if (g < 0.028 && env.intensity === "full") {
      const y = ((Math.sin(time * 13.7) + 1) / 2) * height;
      ctx.globalAlpha = 0.2;
      ctx.fillStyle = Math.sin(time * 31) > 0 ? CYAN : MAGENTA;
      ctx.fillRect(0, y, width, 2 + (g / 0.028) * 5);
      ctx.globalAlpha = 1;
    }
  },

  click(pool, x, y) {
    sparkBurst(pool, x, y, [CYAN, MAGENTA, AMBER], 14, 320);
    pool.emit({ x, y, ttl: 0.35, size: 30, color: rgba(CYAN, 0.9), shape: "ring", b: 1.5 });
  },

  action(kind, pool, x, y) {
    switch (kind) {
      case "like":
        // Broadcast signal: stacked rings + rising sparks.
        for (let i = 0; i < 3; i += 1) {
          pool.emit({
            x,
            y,
            ttl: 0.55 + i * 0.16,
            size: 34 + i * 22,
            color: rgba(i === 1 ? MAGENTA : CYAN, 0.85),
            shape: "ring",
            b: 2,
          });
        }
        sparkBurst(pool, x, y, [CYAN, AMBER], 10, 260);
        break;
      case "dismiss":
        for (let i = 0; i < 12; i += 1) {
          pool.emit({
            x: x + (Math.random() - 0.5) * 30,
            y: y + (Math.random() - 0.5) * 16,
            vx: (Math.random() - 0.5) * 340,
            vy: -40 - Math.random() * 60,
            ay: 420,
            ttl: 0.35 + Math.random() * 0.25,
            size: 4 + Math.random() * 5,
            rot: Math.random() * Math.PI,
            spin: (Math.random() - 0.5) * 10,
            color: i % 3 === 0 ? MAGENTA : "#ff5470",
            shape: "square",
          });
        }
        break;
      case "confirm":
        sparkBurst(pool, x, y, ["#38ef7d", CYAN], 12, 300);
        pool.emit({ x, y, ttl: 0.4, size: 34, color: rgba("#38ef7d", 0.9), shape: "ring", b: 2 });
        break;
      case "error":
        pool.emit({ x, y, ttl: 0.45, size: 40, color: rgba("#ff5470", 0.9), shape: "ring", b: 3 });
        sparkBurst(pool, x, y, ["#ff5470", AMBER], 8, 220);
        break;
    }
  },
};

export const retroTheme: ThemeDefinition = {
  id: "retro",
  label: "复古电子",
  tagline: "CRT 荧光与霓虹地平线",
  palette: { accent: CYAN, accent2: MAGENTA, ink: "#d7fbff" },
  fx,
};
