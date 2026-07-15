// Steam 页面风 (storefront dark blue): deep navy gradients, capsule cards,
// soft cyan hover glow, wishlist-style pulses. Original palette inspired by
// the storefront mood — no Valve assets.

import { rgba } from "../proc";
import type { ParticleEmitter, ThemeFx } from "../../fx/types";
import type { ThemeDefinition } from "../types";

const BLUE = "#66c0f4";
const DEEP = "#1b2838";
const GREEN = "#a4d007";

function pulse(pool: ParticleEmitter, x: number, y: number, color: string): void {
  pool.emit({ x, y, ttl: 0.5, size: 36, color: rgba(color, 0.75), shape: "ring", b: 1.5 });
  pool.emit({ x, y, ttl: 0.45, size: 18, color: rgba(color, 0.5), shape: "glow" });
}

const fx: ThemeFx = {
  drawAmbient(ctx, env) {
    const { width, height, time } = env;
    // Two large soft light blobs slowly orbiting, like the store's hero glow.
    const blobs = [
      { cx: 0.22 + 0.06 * Math.sin(time * 0.07), cy: 0.18 + 0.05 * Math.cos(time * 0.05), r: 0.5, c: "#2a475e" },
      { cx: 0.85 + 0.05 * Math.cos(time * 0.06), cy: 0.75 + 0.06 * Math.sin(time * 0.08), r: 0.55, c: "#173047" },
    ];
    for (const blob of blobs) {
      const r = blob.r * Math.max(width, height);
      const g = ctx.createRadialGradient(
        blob.cx * width,
        blob.cy * height,
        0,
        blob.cx * width,
        blob.cy * height,
        r,
      );
      g.addColorStop(0, rgba(blob.c, 0.5));
      g.addColorStop(1, "rgba(0,0,0,0)");
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, width, height);
    }
  },

  ambientSpawn(pool, env) {
    // Sparse drifting dust motes catching the light.
    if (pool.liveCount() < (env.intensity === "low" ? 10 : 26) && Math.random() < 0.3) {
      pool.emit({
        x: Math.random() * env.width,
        y: env.height + 8,
        vy: -12 - Math.random() * 16,
        vx: (Math.random() - 0.5) * 8,
        ttl: 9 + Math.random() * 6,
        size: 1 + Math.random() * 1.8,
        color: rgba(BLUE, 0.4),
        shape: "dot",
      });
    }
  },

  click(pool, x, y) {
    pulse(pool, x, y, BLUE);
    for (let i = 0; i < 6; i += 1) {
      const angle = Math.random() * Math.PI * 2;
      const speed = 60 + Math.random() * 110;
      pool.emit({
        x,
        y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        drag: 3,
        ttl: 0.4 + Math.random() * 0.2,
        size: 2 + Math.random() * 2,
        color: rgba(BLUE, 0.9),
        shape: "dot",
      });
    }
  },

  action(kind, pool, x, y) {
    switch (kind) {
      case "like":
        // Wishlist-added: green confirmation pulse + rising plus glints.
        pulse(pool, x, y, GREEN);
        for (let i = 0; i < 5; i += 1) {
          pool.emit({
            x: x + (Math.random() - 0.5) * 40,
            y,
            vy: -60 - Math.random() * 40,
            drag: 1,
            ttl: 0.6 + Math.random() * 0.3,
            size: 6,
            color: GREEN,
            shape: "steam-plus",
          });
        }
        break;
      case "dismiss":
        pool.emit({ x, y, ttl: 0.4, size: 30, color: rgba("#c15755", 0.7), shape: "ring", b: 1.5 });
        for (let i = 0; i < 8; i += 1) {
          pool.emit({
            x,
            y,
            vx: (Math.random() - 0.5) * 180,
            vy: (Math.random() - 0.5) * 60,
            drag: 2.4,
            ttl: 0.35,
            size: 2.5,
            color: rgba("#c15755", 0.8),
            shape: "dot",
          });
        }
        break;
      case "confirm":
        pulse(pool, x, y, GREEN);
        break;
      case "error":
        pulse(pool, x, y, "#c15755");
        break;
    }
  },

  shapes: {
    "steam-plus": (ctx, p, t) => {
      const s = p.size * (1 - t * 0.3);
      ctx.globalAlpha = t < 0.6 ? 1 : Math.max(0, 1 - (t - 0.6) / 0.4);
      ctx.strokeStyle = p.color;
      ctx.lineWidth = 2;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(p.x - s, p.y);
      ctx.lineTo(p.x + s, p.y);
      ctx.moveTo(p.x, p.y - s);
      ctx.lineTo(p.x, p.y + s);
      ctx.stroke();
    },
  },
};

export const steamTheme: ThemeDefinition = {
  id: "steam",
  label: "Steam 商店",
  tagline: "深蓝商店与柔光胶囊",
  palette: { accent: BLUE, accent2: GREEN, ink: "#c7d5e0" },
  fx,
};

export const STEAM_COLORS = { BLUE, DEEP, GREEN };
