import { describe, expect, it } from "vitest";
import { ParticlePool } from "../src/fx/pool";

describe("ParticlePool", () => {
  it("never exceeds capacity", () => {
    const pool = new ParticlePool(16);
    for (let i = 0; i < 200; i += 1) {
      pool.emit({ x: 0, y: 0, ttl: 10, shape: "dot" });
    }
    expect(pool.liveCount()).toBeLessThanOrEqual(16);
  });

  it("expires particles after their ttl", () => {
    const pool = new ParticlePool(8);
    pool.emit({ x: 0, y: 0, ttl: 0.5, shape: "dot" });
    expect(pool.liveCount()).toBe(1);
    pool.step(0.3);
    expect(pool.liveCount()).toBe(1);
    pool.step(0.3);
    expect(pool.liveCount()).toBe(0);
  });

  it("integrates velocity and acceleration", () => {
    const pool = new ParticlePool(4);
    pool.emit({ x: 0, y: 0, vx: 10, vy: 0, ay: 100, ttl: 5, shape: "dot" });
    pool.step(1);
    let captured: { x: number; y: number; vy: number } | null = null;
    pool.forEachLive((p) => {
      captured = { x: p.x, y: p.y, vy: p.vy };
    });
    expect(captured).not.toBeNull();
    expect(captured!.x).toBeCloseTo(10, 5);
    expect(captured!.vy).toBeCloseTo(100, 5);
    expect(captured!.y).toBeCloseTo(100, 5);
  });

  it("counts live particles by shape prefix", () => {
    const pool = new ParticlePool(16);
    pool.emit({ x: 0, y: 0, ttl: 5, shape: "wafu-petal" });
    pool.emit({ x: 0, y: 0, ttl: 5, shape: "wafu-maple" });
    pool.emit({ x: 0, y: 0, ttl: 5, shape: "dot" });
    expect(pool.liveCountOf("wafu-")).toBe(2);
    expect(pool.liveCountOf("dot")).toBe(1);
  });

  it("recycles the oldest slot when saturated without leaking counts", () => {
    const pool = new ParticlePool(4);
    for (let i = 0; i < 4; i += 1) pool.emit({ x: i, y: 0, ttl: 5, shape: "dot" });
    expect(pool.liveCount()).toBe(4);
    // Force recycling; live count must stay bounded and correct.
    pool.emit({ x: 99, y: 0, ttl: 5, shape: "dot" });
    expect(pool.liveCount()).toBe(4);
    pool.clear();
    expect(pool.liveCount()).toBe(0);
  });
});
