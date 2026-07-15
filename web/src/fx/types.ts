// Effect engine shared types.
//
// Themes contribute effects through a ThemeFx module:
// - drawAmbient / ambientSpawn run on the background layer (below the UI)
// - click / action spawn particles on the overlay layer (above the UI)
// - shapes registers custom particle renderers
// The engine owns scheduling, pooling, DPR, visibility and intensity policy.

export type FxIntensity = "off" | "low" | "full";

export type ActionKind = "like" | "dismiss" | "confirm" | "error";

export interface Particle {
  alive: boolean;
  x: number;
  y: number;
  vx: number;
  vy: number;
  ax: number;
  ay: number;
  /** Velocity damping per second (0 = none, 1 = strong). */
  drag: number;
  /** Remaining life in seconds. */
  life: number;
  /** Total life in seconds. */
  ttl: number;
  size: number;
  rot: number;
  spin: number;
  color: string;
  /** Renderer key resolved against theme + builtin shapes. */
  shape: string;
  /** Free-form per-particle scratch values (wobble phases etc.). */
  a: number;
  b: number;
  c: number;
}

export type ParticleSpec = Partial<Omit<Particle, "alive" | "life">> & {
  x: number;
  y: number;
  ttl: number;
  shape: string;
};

export interface ParticleEmitter {
  emit(spec: ParticleSpec): void;
  /** Number of live particles (for budget checks in ambient spawners). */
  liveCount(): number;
  /** Live particles matching a shape prefix (e.g. petals only). */
  liveCountOf(prefix: string): number;
}

/** Draws one particle. t is normalized age in [0,1]. */
export type ShapeRenderer = (
  ctx: CanvasRenderingContext2D,
  p: Particle,
  t: number,
) => void;

export interface FxPalette {
  /** Primary accent used by spark/ring effects. */
  accent: string;
  /** Secondary accent for mixed bursts. */
  accent2: string;
  /** Base ink/foreground color. */
  ink: string;
}

export interface AmbientContext {
  width: number;
  height: number;
  /** Elapsed engine time in seconds. */
  time: number;
  /** Frame delta in seconds (clamped). */
  dt: number;
  intensity: Exclude<FxIntensity, "off">;
}

export interface ThemeFx {
  /** Procedural background drawn under ambient particles each frame. */
  drawAmbient?: (ctx: CanvasRenderingContext2D, env: AmbientContext) => void;
  /** Keep the ambient particle field populated (petals, embers, ...). */
  ambientSpawn?: (pool: ParticleEmitter, env: AmbientContext) => void;
  /** Pointer-down feedback at (x, y). */
  click?: (pool: ParticleEmitter, x: number, y: number, palette: FxPalette) => void;
  /** Semantic action feedback (like / dismiss / confirm / error). */
  action?: (
    kind: ActionKind,
    pool: ParticleEmitter,
    x: number,
    y: number,
    palette: FxPalette,
  ) => void;
  /** Theme-specific particle renderers. */
  shapes?: Record<string, ShapeRenderer>;
}
