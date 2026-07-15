// 极简现代白 (minimal-white): restraint is the effect. Hairline geometry,
// one accent, precise expanding-ring feedback. Almost no ambient motion.

import { rgba } from "../proc";
import type { ThemeFx } from "../../fx/types";
import type { ThemeDefinition } from "../types";

const INK = "#17181c";
const ACCENT = "#2456ff";

const fx: ThemeFx = {
  drawAmbient(ctx, env) {
    const { width, height, time } = env;
    // A single slow "breath" line drifting down the page — barely there.
    const y = height * (0.18 + 0.64 * ((Math.sin(time * 0.05) + 1) / 2));
    const grad = ctx.createLinearGradient(0, y, width, y);
    grad.addColorStop(0, "rgba(23,24,28,0)");
    grad.addColorStop(0.5, "rgba(23,24,28,0.05)");
    grad.addColorStop(1, "rgba(23,24,28,0)");
    ctx.strokeStyle = grad;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
    ctx.stroke();
  },

  click(pool, x, y) {
    // One crisp ring + one hairline echo. Nothing else.
    pool.emit({ x, y, ttl: 0.45, size: 34, color: rgba(INK, 0.55), shape: "ring", b: 1 });
    pool.emit({ x, y, ttl: 0.65, size: 52, color: rgba(INK, 0.22), shape: "ring", b: 0.75 });
  },

  action(kind, pool, x, y) {
    switch (kind) {
      case "like":
        pool.emit({ x, y, ttl: 0.5, size: 30, color: rgba(ACCENT, 0.8), shape: "ring", b: 1.25 });
        pool.emit({ x, y, ttl: 0.55, size: 12, color: ACCENT, shape: "min-check" });
        break;
      case "dismiss":
        pool.emit({ x, y, ttl: 0.45, size: 12, color: rgba(INK, 0.7), shape: "min-cross" });
        pool.emit({ x, y, ttl: 0.4, size: 26, color: rgba(INK, 0.3), shape: "ring", b: 1 });
        break;
      case "confirm":
        pool.emit({ x, y, ttl: 0.55, size: 14, color: ACCENT, shape: "min-check" });
        break;
      case "error":
        pool.emit({ x, y, ttl: 0.5, size: 14, color: "#d43737", shape: "min-cross" });
        pool.emit({ x, y, ttl: 0.5, size: 30, color: rgba("#d43737", 0.5), shape: "ring", b: 1 });
        break;
    }
  },

  shapes: {
    // Check mark drawn stroke-by-stroke with an eased reveal.
    "min-check": (ctx, p, t) => {
      const s = p.size;
      const reveal = Math.min(1, t / 0.6);
      ctx.globalAlpha = t < 0.7 ? 1 : Math.max(0, 1 - (t - 0.7) / 0.3);
      ctx.strokeStyle = p.color;
      ctx.lineWidth = 2;
      ctx.lineCap = "round";
      ctx.beginPath();
      const x0 = p.x - s;
      const y0 = p.y;
      const x1 = p.x - s * 0.25;
      const y1 = p.y + s * 0.7;
      const x2 = p.x + s;
      const y2 = p.y - s * 0.7;
      if (reveal <= 0.4) {
        const k = reveal / 0.4;
        ctx.moveTo(x0, y0);
        ctx.lineTo(x0 + (x1 - x0) * k, y0 + (y1 - y0) * k);
      } else {
        const k = (reveal - 0.4) / 0.6;
        ctx.moveTo(x0, y0);
        ctx.lineTo(x1, y1);
        ctx.lineTo(x1 + (x2 - x1) * k, y1 + (y2 - y1) * k);
      }
      ctx.stroke();
    },
    "min-cross": (ctx, p, t) => {
      const s = p.size * (0.8 + 0.2 * t);
      ctx.globalAlpha = t < 0.6 ? 1 : Math.max(0, 1 - (t - 0.6) / 0.4);
      ctx.strokeStyle = p.color;
      ctx.lineWidth = 2;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(p.x - s, p.y - s);
      ctx.lineTo(p.x + s, p.y + s);
      ctx.moveTo(p.x + s, p.y - s);
      ctx.lineTo(p.x - s, p.y + s);
      ctx.stroke();
    },
  },
};

export const minimalTheme: ThemeDefinition = {
  id: "minimal",
  label: "极简白线",
  tagline: "留白、细线与克制的几何",
  palette: { accent: ACCENT, accent2: INK, ink: INK },
  fx,
};
