import type { GameCard } from "../../types";

export function getDisplayedGameScore(
  game: Pick<GameCard, "aiScore" | "recommendationScore">,
) {
  const hasAiScore = typeof game.aiScore === "number" && Number.isFinite(game.aiScore);
  const value = hasAiScore ? game.aiScore! : game.recommendationScore;

  return {
    hasAiScore,
    label: hasAiScore ? "综合推荐" : "推荐值",
    value,
  };
}
