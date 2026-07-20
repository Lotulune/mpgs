import type { PublicVoter } from "../api/types";

export function Facepile({
  voters,
  omittedCount,
  total,
}: {
  voters: PublicVoter[];
  omittedCount: number;
  total: number;
}) {
  const mobileOmittedCount = omittedCount + Math.max(0, voters.length - 3);
  return (
    <div className="facepile" aria-label={`共 ${total} 人想玩`}>
      {voters.map((voter) => (
        <img key={`${voter.display_name}:${voter.avatar_url}`} src={voter.avatar_url} alt={voter.display_name} />
      ))}
      {omittedCount > 0 && <span className="facepile-more desktop-only" aria-label={`另有 ${omittedCount} 人`}>+{omittedCount}</span>}
      {mobileOmittedCount > 0 && <span className="facepile-more mobile-only" aria-label={`另有 ${mobileOmittedCount} 人`}>+{mobileOmittedCount}</span>}
    </div>
  );
}
