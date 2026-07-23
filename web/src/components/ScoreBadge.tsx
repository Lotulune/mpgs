// Recommendation fit-score badge (.score-badge).

import { formatPercent } from "../app/format";

export function ScoreBadge({ score }: { score: number | null }) {
  return (
    <span className="score-badge" title="综合适配分">
      {formatPercent(score)}
    </span>
  );
}
