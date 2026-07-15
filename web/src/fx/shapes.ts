// Builtin particle shape renderers shared by all themes.

import type { Particle, ShapeRenderer } from "./types";

function fade(t: number): number {
  // Smooth fade out over the last 40% of life.
  return t < 0.6 ? 1 : Math.max(0, 1 - (t - 0.6) / 0.4);
}

const dot: ShapeRenderer = (ctx, p, t) => {
  ctx.globalAlpha = fade(t);
  ctx.fillStyle = p.color;
  ctx.beginPath();
  ctx.arc(p.x, p.y, p.size * (1 - t * 0.4), 0, Math.PI * 2);
  ctx.fill();
};

const spark: ShapeRenderer = (ctx, p, t) => {
  // Short line segment along the velocity vector.
  const len = p.size * (1.6 - t);
  const mag = Math.hypot(p.vx, p.vy) || 1;
  const nx = p.vx / mag;
  const ny = p.vy / mag;
  ctx.globalAlpha = fade(t);
  ctx.strokeStyle = p.color;
  ctx.lineWidth = Math.max(1, p.size * 0.28 * (1 - t));
  ctx.beginPath();
  ctx.moveTo(p.x - nx * len, p.y - ny * len);
  ctx.lineTo(p.x, p.y);
  ctx.stroke();
};

const ring: ShapeRenderer = (ctx, p, t) => {
  // Expanding circle outline; p.size is the final radius.
  const r = p.size * easeOut(t);
  ctx.globalAlpha = (1 - t) * 0.9;
  ctx.strokeStyle = p.color;
  ctx.lineWidth = Math.max(0.75, p.b || 1.5) * (1 - t * 0.6);
  ctx.beginPath();
  ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
  ctx.stroke();
};

const square: ShapeRenderer = (ctx, p, t) => {
  const s = p.size * (1 - t * 0.5);
  ctx.globalAlpha = fade(t);
  ctx.fillStyle = p.color;
  ctx.save();
  ctx.translate(p.x, p.y);
  ctx.rotate(p.rot);
  ctx.fillRect(-s / 2, -s / 2, s, s);
  ctx.restore();
};

/** Hard-edged pixel square (no rotation, snapped) for the MC look. */
const pixel: ShapeRenderer = (ctx, p, t) => {
  const s = Math.max(2, Math.round(p.size * (1 - t * 0.35)));
  ctx.globalAlpha = t < 0.75 ? 1 : Math.max(0, 1 - (t - 0.75) / 0.25);
  ctx.fillStyle = p.color;
  ctx.fillRect(Math.round(p.x - s / 2), Math.round(p.y - s / 2), s, s);
};

const glow: ShapeRenderer = (ctx, p, t) => {
  const r = p.size * (1 + t * 0.6);
  const g = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, r);
  g.addColorStop(0, p.color);
  g.addColorStop(1, "rgba(0,0,0,0)");
  ctx.globalAlpha = (1 - t) * 0.55;
  ctx.fillStyle = g;
  ctx.beginPath();
  ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
  ctx.fill();
};

export function easeOut(t: number): number {
  return 1 - (1 - t) * (1 - t) * (1 - t);
}

export const BUILTIN_SHAPES: Record<string, ShapeRenderer> = {
  dot,
  spark,
  ring,
  square,
  pixel,
  glow,
};

export function renderParticle(
  ctx: CanvasRenderingContext2D,
  p: Particle,
  shapes: Record<string, ShapeRenderer>,
): void {
  const t = 1 - p.life / p.ttl;
  const renderer = shapes[p.shape] ?? BUILTIN_SHAPES[p.shape] ?? dot;
  renderer(ctx, p, Math.min(1, Math.max(0, t)));
}
