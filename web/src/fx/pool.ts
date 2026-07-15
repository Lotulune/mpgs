// Fixed-capacity particle pool. No allocation during animation frames.

import type { Particle, ParticleEmitter, ParticleSpec } from "./types";

function blankParticle(): Particle {
  return {
    alive: false,
    x: 0,
    y: 0,
    vx: 0,
    vy: 0,
    ax: 0,
    ay: 0,
    drag: 0,
    life: 0,
    ttl: 1,
    size: 4,
    rot: 0,
    spin: 0,
    color: "#ffffff",
    shape: "dot",
    a: 0,
    b: 0,
    c: 0,
  };
}

export class ParticlePool implements ParticleEmitter {
  readonly capacity: number;
  private readonly items: Particle[];
  private cursor = 0;
  private live = 0;

  constructor(capacity: number) {
    this.capacity = capacity;
    this.items = Array.from({ length: capacity }, blankParticle);
  }

  emit(spec: ParticleSpec): void {
    // Scan from cursor for a dead slot; if saturated, recycle the oldest slot.
    let slot: Particle | null = null;
    for (let i = 0; i < this.capacity; i += 1) {
      const idx = (this.cursor + i) % this.capacity;
      const candidate = this.items[idx];
      if (candidate && !candidate.alive) {
        slot = candidate;
        this.cursor = (idx + 1) % this.capacity;
        break;
      }
    }
    if (!slot) {
      slot = this.items[this.cursor] ?? null;
      this.cursor = (this.cursor + 1) % this.capacity;
      if (!slot) return;
      if (slot.alive) this.live -= 1;
    }
    const p = slot;
    p.alive = true;
    p.x = spec.x;
    p.y = spec.y;
    p.vx = spec.vx ?? 0;
    p.vy = spec.vy ?? 0;
    p.ax = spec.ax ?? 0;
    p.ay = spec.ay ?? 0;
    p.drag = spec.drag ?? 0;
    p.ttl = Math.max(spec.ttl, 0.016);
    p.life = p.ttl;
    p.size = spec.size ?? 4;
    p.rot = spec.rot ?? 0;
    p.spin = spec.spin ?? 0;
    p.color = spec.color ?? "#ffffff";
    p.shape = spec.shape;
    p.a = spec.a ?? 0;
    p.b = spec.b ?? 0;
    p.c = spec.c ?? 0;
    this.live += 1;
  }

  liveCount(): number {
    return this.live;
  }

  liveCountOf(prefix: string): number {
    let count = 0;
    for (const p of this.items) {
      if (p.alive && p.shape.startsWith(prefix)) count += 1;
    }
    return count;
  }

  /** Advance physics; kills expired particles. */
  step(dt: number): void {
    for (const p of this.items) {
      if (!p.alive) continue;
      p.life -= dt;
      if (p.life <= 0) {
        p.alive = false;
        this.live -= 1;
        continue;
      }
      p.vx += p.ax * dt;
      p.vy += p.ay * dt;
      if (p.drag > 0) {
        const keep = Math.max(0, 1 - p.drag * dt);
        p.vx *= keep;
        p.vy *= keep;
      }
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.rot += p.spin * dt;
    }
  }

  forEachLive(fn: (p: Particle) => void): void {
    for (const p of this.items) {
      if (p.alive) fn(p);
    }
  }

  clear(): void {
    for (const p of this.items) p.alive = false;
    this.live = 0;
  }
}
