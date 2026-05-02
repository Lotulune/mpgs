export type DemoStatus =
  | "demo_only"
  | "released_with_demo"
  | "released"
  | "unknown";

export type ReleaseBucket = "new" | "classic";

export interface GameRecommendationFacts {
  appid: number;
  name: string;
  releaseDate?: string | null;
  positiveReviewPct?: number | null;
  totalReviews?: number | null;
  currentPlayers?: number | null;
  multiplayerModes: string[];
  demoStatus: DemoStatus;
  aiScore?: number | null;
}

export function scoreGame(
  facts: GameRecommendationFacts,
  today: Date = new Date(),
): number {
  if (typeof facts.aiScore === "number" && Number.isFinite(facts.aiScore)) {
    return roundOne(clamp(facts.aiScore, 0, 100));
  }

  const reviewQuality = lightweightReviewQuality(facts);
  const multiplayerFit = lightweightMultiplayerFit(facts.multiplayerModes);
  const freshness = freshnessScore(facts.releaseDate, today);
  const discoveryValue = lightweightDiscoveryValue(
    facts,
    reviewQuality,
    freshness,
  );
  const confidence = lightweightConfidence(facts);
  const uncertaintyPenalty = (1 - confidence) * 10;
  const preanalysisPenalty = 4;

  const lightweightQualityProxy =
    0.45 * reviewQuality +
    0.3 * multiplayerFit +
    0.15 * freshness +
    0.1 * discoveryValue;

  return roundOne(
    clamp(
      0.55 * lightweightQualityProxy +
        0.2 * multiplayerFit +
        0.15 * discoveryValue +
        0.1 * freshness -
        uncertaintyPenalty -
        preanalysisPenalty,
      0,
      100,
    ),
  );
}

export function bucketGame(
  facts: Pick<GameRecommendationFacts, "releaseDate">,
  today: Date = new Date(),
): ReleaseBucket {
  const days = daysSinceRelease(facts.releaseDate, today);
  return days !== null && days >= 0 && days <= 30 ? "new" : "classic";
}

function lightweightReviewQuality(facts: GameRecommendationFacts): number {
  const rawPositiveRate = clamp(facts.positiveReviewPct ?? 0, 0, 100) / 100;
  const totalReviews = facts.totalReviews ?? 0;
  const positive = totalReviews * rawPositiveRate;
  const bayesPositiveRate = (positive + 35) / (totalReviews + 35 + 15);
  const confidence = 1 - Math.exp(-totalReviews / 120);

  return clamp(
    100 * (bayesPositiveRate * 0.85 + rawPositiveRate * 0.15) * confidence +
      55 * (1 - confidence),
    0,
    100,
  );
}

function lightweightMultiplayerFit(modes: string[]): number {
  if (modes.length === 0) return 25;

  const normalized = modes.join(" ").toLowerCase();
  let score = 48;
  if (normalized.includes("co-op") || normalized.includes("cooperative")) {
    score += 18;
  }
  if (normalized.includes("online") || normalized.includes("lan")) {
    score += 12;
  }
  if (normalized.includes("local") || normalized.includes("split screen")) {
    score += 12;
  }
  if (normalized.includes("pvp")) {
    score += 8;
  }
  if (modes.length >= 2) {
    score += 5;
  }

  return clamp(score, 0, 100);
}

function freshnessScore(releaseDate: string | null | undefined, today: Date) {
  const days = daysSinceRelease(releaseDate, today);
  if (days === null) return 35;
  if (days >= 0 && days <= 30) return 100;
  if (days <= 90) return 75;
  if (days <= 365) return 45;
  return 25;
}

function lightweightDiscoveryValue(
  facts: GameRecommendationFacts,
  reviewQuality: number,
  freshness: number,
) {
  const positiveReviewPct = facts.positiveReviewPct ?? 0;
  const totalReviews = facts.totalReviews ?? 0;
  const currentPlayers = facts.currentPlayers ?? 0;
  const sleeperScore =
    (reviewQuality >= 78 && totalReviews < 500 ? 25 : 0) +
    (currentPlayers < 300 && positiveReviewPct >= 85 ? 20 : 0) +
    (facts.demoStatus === "demo_only" || facts.demoStatus === "released_with_demo"
      ? 15
      : 0) +
    (freshness >= 75 ? 10 : 0);

  const demoPotential =
    facts.demoStatus === "demo_only"
      ? 60
      : facts.demoStatus === "released_with_demo"
        ? 45
        : facts.demoStatus === "released"
          ? 15
          : 10;

  return clamp(
    0.45 * freshness + 0.4 * sleeperScore + 0.15 * demoPotential,
    0,
    100,
  );
}

function lightweightConfidence(facts: GameRecommendationFacts) {
  const reviewConfidence = 1 - Math.exp(-(facts.totalReviews ?? 0) / 120);
  const modeConfidence = Math.min(facts.multiplayerModes.length / 3, 1);
  const activityConfidence =
    typeof facts.currentPlayers === "number" ? 1 : 0.35;

  return clamp(
    0.6 * reviewConfidence + 0.25 * activityConfidence + 0.15 * modeConfidence,
    0,
    1,
  );
}

function daysSinceRelease(
  releaseDate: string | null | undefined,
  today: Date,
): number | null {
  if (!releaseDate) return null;
  const release = new Date(`${releaseDate}T00:00:00Z`);
  if (Number.isNaN(release.getTime())) return null;
  const todayUtc = Date.UTC(
    today.getUTCFullYear(),
    today.getUTCMonth(),
    today.getUTCDate(),
  );
  return Math.floor((todayUtc - release.getTime()) / 86_400_000);
}

function roundOne(value: number): number {
  return Math.round(value * 10) / 10;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
