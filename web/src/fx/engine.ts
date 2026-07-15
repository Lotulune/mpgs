// FxEngine: owns two full-window canvases and one rAF loop.
//
//   ambient layer  — behind the UI (z-index below app root): procedural theme
//                    backdrop + slow ambient particles (petals, embers, ...)
//   overlay layer  — above the UI, pointer-events: none: click bursts and
//                    semantic action effects
//
// Policies: DPR-aware sizing, pause when the tab is hidden, hard particle caps,
// clamped dt, honors prefers-reduced-motion via the intensity setting.

import { ParticlePool } from "./pool";
import { BUILTIN_SHAPES, renderParticle } from "./shapes";
import type {
  ActionKind,
  AmbientContext,
  FxIntensity,
  FxPalette,
  ShapeRenderer,
  ThemeFx,
} from "./types";

const AMBIENT_CAP_FULL = 160;
const AMBIENT_CAP_LOW = 48;
const OVERLAY_CAP = 320;
const MAX_DT = 1 / 20;

export class FxEngine {
  private ambientCanvas: HTMLCanvasElement | null = null;
  private overlayCanvas: HTMLCanvasElement | null = null;
  private ambientCtx: CanvasRenderingContext2D | null = null;
  private overlayCtx: CanvasRenderingContext2D | null = null;

  private readonly ambientPool = new ParticlePool(AMBIENT_CAP_FULL);
  private readonly overlayPool = new ParticlePool(OVERLAY_CAP);

  private themeFx: ThemeFx = {};
  private shapes: Record<string, ShapeRenderer> = { ...BUILTIN_SHAPES };
  private palette: FxPalette = { accent: "#4da3ff", accent2: "#9d6bff", ink: "#e8e8ea" };

  private intensity: FxIntensity = "full";
  private running = false;
  private rafId = 0;
  private lastTs = 0;
  private time = 0;
  private width = 0;
  private height = 0;
  private dpr = 1;

  private readonly onResize = () => this.resize();
  private readonly onVisibility = () => {
    if (document.hidden) {
      this.stopLoop();
    } else {
      this.startLoop();
    }
  };

  attach(ambient: HTMLCanvasElement, overlay: HTMLCanvasElement): void {
    this.ambientCanvas = ambient;
    this.overlayCanvas = overlay;
    this.ambientCtx = ambient.getContext("2d");
    this.overlayCtx = overlay.getContext("2d");
    this.resize();
    window.addEventListener("resize", this.onResize);
    document.addEventListener("visibilitychange", this.onVisibility);
    this.startLoop();
  }

  detach(): void {
    this.stopLoop();
    window.removeEventListener("resize", this.onResize);
    document.removeEventListener("visibilitychange", this.onVisibility);
    this.ambientCanvas = null;
    this.overlayCanvas = null;
    this.ambientCtx = null;
    this.overlayCtx = null;
  }

  setThemeFx(fx: ThemeFx, palette: FxPalette): void {
    this.themeFx = fx;
    this.palette = palette;
    this.shapes = { ...BUILTIN_SHAPES, ...(fx.shapes ?? {}) };
    // Theme switch: clear stale particles so effects never bleed across themes.
    this.ambientPool.clear();
    this.overlayPool.clear();
    this.clearCanvas(this.ambientCtx, this.ambientCanvas);
    this.clearCanvas(this.overlayCtx, this.overlayCanvas);
  }

  setIntensity(intensity: FxIntensity): void {
    this.intensity = intensity;
    if (intensity === "off") {
      this.ambientPool.clear();
      this.overlayPool.clear();
      this.clearCanvas(this.ambientCtx, this.ambientCanvas);
      this.clearCanvas(this.overlayCtx, this.overlayCanvas);
    }
  }

  getIntensity(): FxIntensity {
    return this.intensity;
  }

  /** Pointer-down feedback. Coordinates are CSS pixels relative to the viewport. */
  click(x: number, y: number): void {
    if (this.intensity === "off") return;
    this.themeFx.click?.(this.overlayPool, x, y, this.palette);
  }

  /** Semantic feedback anchored to an element or point. */
  action(kind: ActionKind, x: number, y: number): void {
    if (this.intensity === "off") return;
    this.themeFx.action?.(kind, this.overlayPool, x, y, this.palette);
  }

  actionAt(kind: ActionKind, element: Element): void {
    const rect = element.getBoundingClientRect();
    this.action(kind, rect.left + rect.width / 2, rect.top + rect.height / 2);
  }

  private resize(): void {
    this.width = window.innerWidth;
    this.height = window.innerHeight;
    this.dpr = Math.min(window.devicePixelRatio || 1, 2);
    for (const canvas of [this.ambientCanvas, this.overlayCanvas]) {
      if (!canvas) continue;
      canvas.width = Math.round(this.width * this.dpr);
      canvas.height = Math.round(this.height * this.dpr);
      canvas.style.width = `${this.width}px`;
      canvas.style.height = `${this.height}px`;
    }
  }

  private startLoop(): void {
    if (this.running) return;
    this.running = true;
    this.lastTs = performance.now();
    const frame = (ts: number) => {
      if (!this.running) return;
      const dt = Math.min(MAX_DT, Math.max(0, (ts - this.lastTs) / 1000));
      this.lastTs = ts;
      this.time += dt;
      this.tick(dt);
      this.rafId = requestAnimationFrame(frame);
    };
    this.rafId = requestAnimationFrame(frame);
  }

  private stopLoop(): void {
    this.running = false;
    cancelAnimationFrame(this.rafId);
  }

  private clearCanvas(
    ctx: CanvasRenderingContext2D | null,
    canvas: HTMLCanvasElement | null,
  ): void {
    if (ctx && canvas) ctx.clearRect(0, 0, canvas.width, canvas.height);
  }

  private tick(dt: number): void {
    const { ambientCtx, overlayCtx } = this;
    if (!ambientCtx || !overlayCtx) return;

    // Narrow to a local so AmbientContext.intensity (never "off") type-checks.
    const activeIntensity = this.intensity;

    // --- ambient layer ---
    ambientCtx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    ambientCtx.clearRect(0, 0, this.width, this.height);
    if (activeIntensity !== "off") {
      const env: AmbientContext = {
        width: this.width,
        height: this.height,
        time: this.time,
        dt,
        intensity: activeIntensity,
      };
      this.themeFx.drawAmbient?.(ambientCtx, env);
      if (this.themeFx.ambientSpawn) {
        const cap = activeIntensity === "low" ? AMBIENT_CAP_LOW : AMBIENT_CAP_FULL;
        if (this.ambientPool.liveCount() < cap) {
          this.themeFx.ambientSpawn(this.ambientPool, env);
        }
      }
      this.ambientPool.step(dt);
      this.ambientPool.forEachLive((p) => {
        renderParticle(ambientCtx, p, this.shapes);
      });
      ambientCtx.globalAlpha = 1;
    }

    // --- overlay layer ---
    overlayCtx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    overlayCtx.clearRect(0, 0, this.width, this.height);
    if (activeIntensity !== "off") {
      this.overlayPool.step(dt);
      this.overlayPool.forEachLive((p) => {
        renderParticle(overlayCtx, p, this.shapes);
      });
      overlayCtx.globalAlpha = 1;
    }
  }
}

export const fxEngine = new FxEngine();

export function preferredIntensity(): FxIntensity {
  if (typeof window.matchMedia === "function") {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return "low";
    }
  }
  return "full";
}
