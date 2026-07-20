import { useEffect, useMemo, useState } from "react";

/** Candidate cover URLs for a Steam app, most specific first. */
export function coverCandidates(appId: number, coverUrl?: string | null): string[] {
  const out: string[] = [];
  const push = (url: string | null | undefined) => {
    const value = url?.trim();
    if (!value || out.includes(value)) return;
    out.push(value);
  };
  push(coverUrl);
  if (appId > 0) {
    // Stable public CDN paths. Hashed shared-asset filenames need appdetails.
    push(`https://cdn.akamai.steamstatic.com/steam/apps/${appId}/header.jpg`);
    push(`https://cdn.cloudflare.steamstatic.com/steam/apps/${appId}/header.jpg`);
    push(`https://steamcdn-a.akamaihd.net/steam/apps/${appId}/header.jpg`);
    push(`https://cdn.akamai.steamstatic.com/steam/apps/${appId}/capsule_616x353.jpg`);
    push(`https://cdn.akamai.steamstatic.com/steam/apps/${appId}/capsule_231x87.jpg`);
  }
  return out;
}

export function steamHeaderCoverUrl(appId: number): string {
  return `https://cdn.akamai.steamstatic.com/steam/apps/${appId}/header.jpg`;
}

export function resolveCoverUrl(appId: number, coverUrl: string | null | undefined): string | null {
  return coverCandidates(appId, coverUrl)[0] ?? null;
}

export function GameMedia({
  coverUrl,
  name,
  appId,
  compact = false,
}: {
  coverUrl: string | null;
  name: string;
  appId?: number;
  compact?: boolean;
}) {
  const candidates = useMemo(
    () => coverCandidates(appId ?? 0, coverUrl),
    [appId, coverUrl],
  );
  const [index, setIndex] = useState(0);

  useEffect(() => {
    setIndex(0);
  }, [candidates]);

  const src = index < candidates.length ? candidates[index] : null;
  if (!src) {
    return (
      <div className={`game-media${compact ? " compact" : ""}`} aria-hidden="true">
        <span className="game-media-fallback">{name.slice(0, 1).toUpperCase()}</span>
      </div>
    );
  }

  return (
    <div className={`game-media${compact ? " compact" : ""}`}>
      <img
        src={src}
        alt={`${name} 封面`}
        loading="lazy"
        decoding="async"
        referrerPolicy="no-referrer"
        onError={() => setIndex((current) => current + 1)}
      />
    </div>
  );
}
