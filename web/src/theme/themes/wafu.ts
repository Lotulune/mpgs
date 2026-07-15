// 樱花枫树和风 (wafu): washi paper, drifting sakura petals and maple leaves,
// ink-wash ripple clicks, vermilion hanko stamp for "like".

import { makeTexture, mulberry32, rgba } from "../proc";
import type { ParticleEmitter, ThemeFx } from "../../fx/types";
import type { ThemeDefinition } from "../types";

const SAKURA = "#f6b8c8";
const SAKURA_DEEP = "#ef93ab";
const MOMIJI = "#d1512d";
const MOMIJI_DEEP = "#a63a1e";
const SUMI = "#3d3a37"; // ink
const SHU = "#c73e3a"; // vermilion seal red

function washiTexture(): string {
  const rng = mulberry32(7);
  return makeTexture(160, (ctx, size) => {
    ctx.fillStyle = "#f7f2e7";
    ctx.fillRect(0, 0, size, size);
    // Paper fibers: short faint strokes in random directions.
    for (let i = 0; i < 420; i += 1) {
      const x = rng() * size;
      const y = rng() * size;
      const len = 2 + rng() * 7;
      const angle = rng() * Math.PI;
      ctx.strokeStyle = rng() > 0.5 ? "rgba(190,178,155,0.16)" : "rgba(222,214,196,0.22)";
      ctx.lineWidth = 0.6;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + Math.cos(angle) * len, y + Math.sin(angle) * len);
      ctx.stroke();
    }
  });
}

function spawnPetal(pool: ParticleEmitter, env: { width: number; height: number }): void {
  const isMaple = Math.random() < 0.3;
  pool.emit({
    x: Math.random() * (env.width + 160) - 80,
    y: -16,
    vx: 12 + Math.random() * 26,
    vy: 26 + Math.random() * 34,
    ttl: 16,
    size: isMaple ? 7 + Math.random() * 5 : 5 + Math.random() * 4,
    rot: Math.random() * Math.PI * 2,
    spin: (Math.random() - 0.5) * 1.6,
    color: isMaple
      ? (Math.random() > 0.5 ? MOMIJI : MOMIJI_DEEP)
      : (Math.random() > 0.5 ? SAKURA : SAKURA_DEEP),
    shape: isMaple ? "wafu-maple" : "wafu-petal",
    a: Math.random() * Math.PI * 2, // sway phase
    b: 0.6 + Math.random() * 1.2, // sway amplitude
  });
}

const fx: ThemeFx = {
  drawAmbient(ctx, env) {
    const { width, height, time } = env;
    // Distant mountain silhouettes in mist (two layered sine ridges).
    const ridge = (base: number, amp: number, freq: number, phase: number, alpha: number) => {
      ctx.beginPath();
      ctx.moveTo(0, height);
      for (let x = 0; x <= width; x += 16) {
        const y =
          base +
          Math.sin((x / width) * Math.PI * freq + phase) * amp +
          Math.sin((x / width) * Math.PI * freq * 2.7 + phase * 1.7) * amp * 0.3;
        ctx.lineTo(x, y);
      }
      ctx.lineTo(width, height);
      ctx.closePath();
      ctx.fillStyle = rgba(SUMI, alpha);
      ctx.fill();
    };
    ridge(height * 0.82, 26, 2.2, 0.5 + time * 0.004, 0.05);
    ridge(height * 0.9, 34, 1.6, 2.1 - time * 0.003, 0.08);
    // Sun disc, faint, upper right.
    ctx.beginPath();
    ctx.arc(width * 0.86, height * 0.16, 46, 0, Math.PI * 2);
    ctx.fillStyle = rgba(SHU, 0.07);
    ctx.fill();
  },

  ambientSpawn(pool, env) {
    const target = env.intensity === "low" ? 18 : 54;
    if (pool.liveCountOf("wafu-") < target && Math.random() < 0.35) {
      spawnPetal(pool, env);
    }
  },

  click(pool, x, y) {
    // Ink-wash ripple: soft dark rings like a brush touching water.
    pool.emit({ x, y, ttl: 0.7, size: 30, color: rgba(SUMI, 0.4), shape: "ring", b: 2.5 });
    pool.emit({ x, y, ttl: 1.0, size: 52, color: rgba(SUMI, 0.2), shape: "ring", b: 1.5 });
    // A few petals scatter from the touch point.
    for (let i = 0; i < 5; i += 1) {
      const angle = Math.random() * Math.PI - Math.PI; // upward bias
      const speed = 60 + Math.random() * 90;
      pool.emit({
        x,
        y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed * 0.6,
        ay: 70,
        drag: 1.4,
        ttl: 0.9 + Math.random() * 0.5,
        size: 4 + Math.random() * 3,
        rot: Math.random() * Math.PI * 2,
        spin: (Math.random() - 0.5) * 6,
        color: Math.random() > 0.4 ? SAKURA : SAKURA_DEEP,
        shape: "wafu-petal",
        a: Math.random() * Math.PI * 2,
        b: 1,
      });
    }
  },

  action(kind, pool, x, y) {
    switch (kind) {
      case "like":
        // Vermilion hanko stamp pressed onto the page.
        pool.emit({ x, y, ttl: 1.1, size: 20, color: SHU, shape: "wafu-hanko" });
        pool.emit({ x, y, ttl: 0.7, size: 40, color: rgba(SHU, 0.35), shape: "ring", b: 2 });
        break;
      case "dismiss":
        // Petals blown away by a gust.
        for (let i = 0; i < 9; i += 1) {
          pool.emit({
            x: x + (Math.random() - 0.5) * 30,
            y: y + (Math.random() - 0.5) * 16,
            vx: 130 + Math.random() * 140,
            vy: -30 + Math.random() * 60,
            drag: 0.9,
            ttl: 0.8 + Math.random() * 0.4,
            size: 4 + Math.random() * 3,
            rot: Math.random() * Math.PI * 2,
            spin: (Math.random() - 0.5) * 8,
            color: [SAKURA, SAKURA_DEEP, MOMIJI][i % 3] ?? SAKURA,
            shape: "wafu-petal",
            a: Math.random() * Math.PI * 2,
            b: 1,
          });
        }
        break;
      case "confirm":
        pool.emit({ x, y, ttl: 0.8, size: 36, color: rgba("#5a7d4f", 0.5), shape: "ring", b: 2 });
        break;
      case "error":
        pool.emit({ x, y, ttl: 0.7, size: 34, color: rgba(SHU, 0.6), shape: "ring", b: 2.5 });
        break;
    }
  },

  shapes: {
    // Sakura petal: rounded teardrop with a notch, swaying as it falls.
    "wafu-petal": (ctx, p, t) => {
      const sway = Math.sin(p.a + t * Math.PI * 2 * p.b) * 0.6;
      ctx.save();
      ctx.translate(p.x + sway * p.size, p.y);
      ctx.rotate(p.rot + sway * 0.5);
      ctx.globalAlpha = t < 0.8 ? 0.9 : Math.max(0, 1 - (t - 0.8) / 0.2) * 0.9;
      ctx.fillStyle = p.color;
      const s = p.size;
      ctx.beginPath();
      ctx.moveTo(0, -s);
      ctx.bezierCurveTo(s * 0.9, -s * 0.5, s * 0.7, s * 0.6, 0, s);
      ctx.bezierCurveTo(-s * 0.7, s * 0.6, -s * 0.9, -s * 0.5, 0, -s);
      ctx.fill();
      // Petal notch highlight.
      ctx.globalAlpha *= 0.5;
      ctx.fillStyle = "#ffffff";
      ctx.beginPath();
      ctx.ellipse(0, -s * 0.35, s * 0.22, s * 0.4, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    },
    // Maple leaf: five-pointed star-ish silhouette.
    "wafu-maple": (ctx, p, t) => {
      const sway = Math.sin(p.a + t * Math.PI * 2 * p.b) * 0.8;
      ctx.save();
      ctx.translate(p.x + sway * p.size, p.y);
      ctx.rotate(p.rot + sway * 0.4);
      ctx.globalAlpha = t < 0.8 ? 0.9 : Math.max(0, 1 - (t - 0.8) / 0.2) * 0.9;
      ctx.fillStyle = p.color;
      const s = p.size;
      ctx.beginPath();
      for (let i = 0; i < 5; i += 1) {
        const angle = -Math.PI / 2 + (i * Math.PI * 2) / 5;
        const tipX = Math.cos(angle) * s;
        const tipY = Math.sin(angle) * s;
        const inAngle = angle + Math.PI / 5;
        const inX = Math.cos(inAngle) * s * 0.42;
        const inY = Math.sin(inAngle) * s * 0.42;
        if (i === 0) ctx.moveTo(tipX, tipY);
        else ctx.lineTo(tipX, tipY);
        ctx.lineTo(inX, inY);
      }
      ctx.closePath();
      ctx.fill();
      ctx.restore();
    },
    // Hanko: square vermilion seal with 「好」 pressed with a slight overshoot.
    "wafu-hanko": (ctx, p, t) => {
      const appear = Math.min(1, t / 0.18);
      const scale = 1.6 - 0.6 * (1 - (1 - appear) * (1 - appear)); // 1.6 -> 1.0 press
      const alpha = t < 0.75 ? appear : Math.max(0, 1 - (t - 0.75) / 0.25);
      const s = p.size * scale;
      ctx.save();
      ctx.translate(p.x, p.y);
      ctx.rotate(-0.08);
      ctx.globalAlpha = alpha * 0.92;
      ctx.fillStyle = p.color;
      const r = s * 0.2;
      ctx.beginPath();
      ctx.moveTo(-s + r, -s);
      ctx.arcTo(s, -s, s, s, r);
      ctx.arcTo(s, s, -s, s, r);
      ctx.arcTo(-s, s, -s, -s, r);
      ctx.arcTo(-s, -s, s, -s, r);
      ctx.closePath();
      ctx.fill();
      ctx.globalAlpha = alpha;
      ctx.fillStyle = "#f7f2e7";
      ctx.font = `700 ${Math.round(s * 1.15)}px "Noto Serif SC", "SimSun", serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText("好", 0, s * 0.06);
      ctx.restore();
    },
  },
};

export const wafuTheme: ThemeDefinition = {
  id: "wafu",
  label: "樱枫和风",
  tagline: "和纸、落樱与朱印",
  palette: { accent: SHU, accent2: SAKURA_DEEP, ink: SUMI },
  fx,
  onActivate(root) {
    root.style.setProperty("--wafu-washi", `url(${washiTexture()})`);
  },
};
