// 樱树和风 (wafu) 2.0: washi paper, a quiet seigaiha band under a single
// gold hairline, a sakura branch silhouetted in the top-right corner, and
// tumbling sakura petals (maple has retired). Ink-wash ripple clicks and a
// vermilion hanko stamp for "like".

import { makeTexture, mulberry32, rgba } from "../proc";
import type { ParticleEmitter, ThemeFx } from "../../fx/types";
import type { ThemeDefinition } from "../types";

const SAKURA = "#f4becb";
const SAKURA_DEEP = "#e89fb2";
const SAKURA_UI = "#be6e84"; // accent-2: muted sakura for UI accents
const SUMI = "#211d17"; // ink
const SHU = "#c53a2a"; // vermilion seal red
const AI = "#6e7b8e"; // muted indigo, seigaiha only
const GOLD = "#b8924a"; // sparse gold flecks

function washiTexture(): string {
  const rng = mulberry32(7);
  return makeTexture(160, (ctx, size) => {
    ctx.fillStyle = "#faf5eb";
    ctx.fillRect(0, 0, size, size);
    // Paper fibers: short faint strokes in random directions.
    for (let i = 0; i < 420; i += 1) {
      const x = rng() * size;
      const y = rng() * size;
      const len = 2 + rng() * 7;
      const angle = rng() * Math.PI;
      ctx.strokeStyle = rng() > 0.5 ? "rgba(190,178,155,0.15)" : "rgba(224,215,196,0.22)";
      ctx.lineWidth = 0.6;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + Math.cos(angle) * len, y + Math.sin(angle) * len);
      ctx.stroke();
    }
  });
}

// 青海波 (seigaiha): overlapping concentric scallop rows in faint indigo.
// 56px square tile; rows sit in each other's gaps so it repeats seamlessly.
function seigaihaTexture(): string {
  const r = 14;
  return makeTexture(r * 4, (ctx, size) => {
    for (let row = -1; row * r < size + r; row += 1) {
      const yBase = row * r;
      const offset = row % 2 === 0 ? 0 : r;
      for (let x = -r; x < size + r; x += r * 2) {
        const cx = x + offset;
        // Faint filled scallop.
        ctx.beginPath();
        ctx.arc(cx, yBase, r, Math.PI, 0);
        ctx.closePath();
        ctx.fillStyle = rgba(AI, 0.045);
        ctx.fill();
        // Concentric ring outlines.
        for (let k = 0; k < 3; k += 1) {
          ctx.beginPath();
          ctx.arc(cx, yBase, r - k * (r / 3) - 0.5, Math.PI, 0);
          ctx.strokeStyle = rgba(AI, 0.13);
          ctx.lineWidth = 1;
          ctx.stroke();
        }
      }
    }
  });
}

// ---------------------------------------------------------------------------
// Sakura branch (ambient layer). Deterministic silhouette anchored to the
// top-right corner, rebuilt only when the viewport size bucket changes.
// Coordinates live in "anchor space": origin at (width, 0), +x goes off-canvas
// right, so negative x reaches inward over the page.
// ---------------------------------------------------------------------------

interface BranchStroke {
  pts: number[]; // flattened polyline [x0, y0, x1, y1, ...]
  w: number;
}

interface Blossom {
  x: number;
  y: number;
  r: number;
  tone: number; // 0 light petal, 1 deep petal
  phase: number; // individual sway offset
  gold: boolean;
}

interface BranchCache {
  key: string;
  strokes: BranchStroke[];
  blossoms: Blossom[];
}

let branchCache: BranchCache | null = null;

function buildBranch(width: number, height: number): BranchCache {
  const key = `${Math.round(width / 64)}:${Math.round(height / 64)}`;
  if (branchCache && branchCache.key === key) return branchCache;
  const rng = mulberry32(20260722);
  const strokes: BranchStroke[] = [];
  const blossoms: Blossom[] = [];

  // Main limb: sweeps in from the top-right corner, leftward and gently down.
  const len = Math.min(width * 0.42, 560);
  const segs = 7;
  let x = 24;
  let y = -10;
  let angle = Math.PI * 0.94;
  const limb = [x, y];
  for (let i = 0; i < segs; i += 1) {
    const step = (len / segs) * (0.85 + rng() * 0.3);
    angle += (rng() - 0.35) * 0.28;
    x += Math.cos(angle) * step;
    y += Math.sin(angle) * step * 0.62 + 3;
    limb.push(x, y);
    // Twig reaching up or down from this joint.
    if (i > 0 && rng() < 0.75) {
      const tAng = angle + (rng() > 0.5 ? -1 : 1) * (0.5 + rng() * 0.5);
      const tLen = 26 + rng() * 52;
      const tx = x + Math.cos(tAng) * tLen;
      const ty = y + Math.sin(tAng) * tLen * 0.7;
      strokes.push({
        pts: [x, y, (x + tx) / 2 + (rng() - 0.5) * 8, (y + ty) / 2 + (rng() - 0.5) * 8, tx, ty],
        w: 1.6 + rng() * 1.2,
      });
      // Blossom cluster at the twig tip.
      const n = 2 + Math.floor(rng() * 3);
      for (let b = 0; b < n; b += 1) {
        blossoms.push({
          x: tx + (rng() - 0.5) * 16,
          y: ty + (rng() - 0.5) * 12,
          r: 2.6 + rng() * 2.6,
          tone: rng(),
          phase: rng() * Math.PI * 2,
          gold: rng() < 0.08,
        });
      }
    }
  }
  strokes.unshift({ pts: limb, w: 4.2 });
  // A few blossoms hugging the main limb itself.
  for (let i = 2; i < limb.length; i += 2) {
    if (rng() < 0.55) {
      blossoms.push({
        x: (limb[i - 2] ?? 0) + (rng() - 0.5) * 10,
        y: (limb[i - 1] ?? 0) + (rng() - 0.5) * 10,
        r: 2.4 + rng() * 2.4,
        tone: rng(),
        phase: rng() * Math.PI * 2,
        gold: rng() < 0.08,
      });
    }
  }
  branchCache = { key, strokes, blossoms };
  return branchCache;
}

function spawnPetal(pool: ParticleEmitter, env: { width: number; height: number }): void {
  const gold = Math.random() < 0.06; // rare gold-flecked petal
  pool.emit({
    x: Math.random() * (env.width + 160) - 80,
    y: -16,
    vx: 10 + Math.random() * 24,
    vy: 20 + Math.random() * 26,
    ttl: 18,
    size: 4.5 + Math.random() * 3.5,
    rot: Math.random() * Math.PI * 2,
    spin: (Math.random() - 0.5) * 1.1,
    color: gold ? GOLD : Math.random() > 0.5 ? SAKURA : SAKURA_DEEP,
    shape: "wafu-petal",
    a: Math.random() * Math.PI * 2, // sway phase
    b: 0.5 + Math.random() * 0.9, // sway speed
    c: Math.random() * Math.PI * 2, // tumble (3D flip) phase
  });
}

const fx: ThemeFx = {
  drawAmbient(ctx, env) {
    const { width, height, time } = env;
    // Faint sun disc, upper left.
    ctx.beginPath();
    ctx.arc(width * 0.12, height * 0.16, 44, 0, Math.PI * 2);
    ctx.fillStyle = rgba(SHU, 0.05);
    ctx.fill();

    // Sakura branch reaching in from the top-right corner, swaying gently.
    const branch = buildBranch(width, height);
    const sway = Math.sin(time * 0.5) * 0.008 + Math.sin(time * 0.13) * 0.004;
    ctx.save();
    ctx.translate(width, 0);
    ctx.rotate(sway);
    ctx.strokeStyle = rgba(SUMI, 0.38);
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    for (const s of branch.strokes) {
      ctx.beginPath();
      ctx.moveTo(s.pts[0] ?? 0, s.pts[1] ?? 0);
      for (let i = 2; i < s.pts.length; i += 2) {
        ctx.lineTo(s.pts[i] ?? 0, s.pts[i + 1] ?? 0);
      }
      ctx.lineWidth = s.w;
      ctx.stroke();
    }
    // Blossoms: five-dot flowers with a vermilion heart.
    for (const b of branch.blossoms) {
      const bx = b.x + Math.sin(time * 0.6 + b.phase) * 1.4;
      const by = b.y + Math.cos(time * 0.45 + b.phase) * 1.0;
      ctx.fillStyle = b.gold ? rgba(GOLD, 0.7) : rgba(b.tone > 0.5 ? SAKURA_DEEP : SAKURA, 0.85);
      for (let p = 0; p < 5; p += 1) {
        const a = (p / 5) * Math.PI * 2 + b.phase;
        ctx.beginPath();
        ctx.arc(bx + Math.cos(a) * b.r * 0.62, by + Math.sin(a) * b.r * 0.62, b.r * 0.52, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.fillStyle = rgba(SHU, 0.5);
      ctx.beginPath();
      ctx.arc(bx, by, b.r * 0.2, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
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
        // Petals blown away by a gust (花吹雪).
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
            color: [SAKURA, SAKURA_DEEP, SAKURA_UI][i % 3] ?? SAKURA,
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
    // Sakura petal: twin-lobed silhouette with the signature notch at the
    // tip, tumbling (fake 3D flip via scaleX) as it drifts down.
    "wafu-petal": (ctx, p, t) => {
      const sway = Math.sin(p.a + t * Math.PI * 2 * p.b);
      const flip = Math.sin(p.c + t * Math.PI * 2 * (p.b * 0.8 + 0.4));
      ctx.save();
      ctx.translate(p.x + sway * p.size * 1.7, p.y);
      ctx.rotate(p.rot + sway * 0.6);
      ctx.scale(0.22 + 0.78 * Math.abs(flip), 1);
      ctx.globalAlpha = t < 0.8 ? 0.85 : Math.max(0, 1 - (t - 0.8) / 0.2) * 0.85;
      ctx.fillStyle = p.color;
      const s = p.size;
      ctx.beginPath();
      ctx.moveTo(0, -s); // stem end
      ctx.bezierCurveTo(s * 0.95, -s * 0.45, s * 0.8, s * 0.35, s * 0.26, s * 0.68);
      ctx.lineTo(0, s * 0.42); // notch cut into the tip
      ctx.lineTo(-s * 0.26, s * 0.68);
      ctx.bezierCurveTo(-s * 0.8, s * 0.35, -s * 0.95, -s * 0.45, 0, -s);
      ctx.fill();
      // Vein highlight.
      ctx.globalAlpha *= 0.45;
      ctx.fillStyle = "#ffffff";
      ctx.beginPath();
      ctx.ellipse(0, -s * 0.3, s * 0.16, s * 0.42, 0, 0, Math.PI * 2);
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
      ctx.fillStyle = "#faf5eb";
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
  label: "樱树和风",
  tagline: "和纸、樱枝与朱印",
  palette: { accent: SHU, accent2: SAKURA_UI, ink: SUMI },
  fx,
  onActivate(root) {
    root.style.setProperty("--wafu-washi", `url(${washiTexture()})`);
    root.style.setProperty("--wafu-seigaiha", `url(${seigaihaTexture()})`);
  },
};
