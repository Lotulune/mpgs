import { useEffect, useMemo, useState } from "react";
import { ApiError } from "../api/client";
import type {
  CommunityFilters,
  CommunityItem,
  CommunityPlatform,
  CommunityReleaseState,
  CommunityResponse,
  CommunitySort,
} from "../api/types";
import { apiClient, playIntentStore } from "../app/runtime";
import { formatAgo, formatCount, formatReleaseDate, releaseStateLabel } from "../app/format";
import { Facepile } from "./Facepile";
import { GameMedia } from "./GameMedia";
import { VoteButton } from "./VoteButton";

export function CommunityScreen({ onOpenGame }: { onOpenGame: (appId: number) => void }) {
  const [sort, setSort] = useState<CommunitySort>("trending");
  const [releaseState, setReleaseState] = useState<CommunityReleaseState | "">("");
  const [demoOnly, setDemoOnly] = useState(false);
  const [platform, setPlatform] = useState<CommunityPlatform | "">("");
  const [partySize, setPartySize] = useState("");
  const [items, setItems] = useState<CommunityItem[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [updatedAt, setUpdatedAt] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const filters = useMemo<CommunityFilters>(() => ({
    ...(releaseState ? { releaseState } : {}),
    ...(demoOnly ? { demoOnly: true } : {}),
    ...(platform ? { platform } : {}),
    ...(partySize && Number(partySize) >= 1 && Number(partySize) <= 64
      ? { partySize: Number(partySize) }
      : {}),
  }), [demoOnly, partySize, platform, releaseState]);

  const apply = (response: CommunityResponse, append: boolean) => {
    setItems((current) => (append ? [...current, ...response.items] : response.items));
    setNextCursor(response.next_cursor);
    setUpdatedAt(response.data_updated_at_ms);
  };

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    void apiClient
      .community(sort, filters)
      .then((result) => {
        if (!cancelled) apply(result.data, false);
      })
      .catch((cause: unknown) => {
        if (cancelled) return;
        setError(cause instanceof ApiError && cause.offline ? "当前离线，无法加载社区榜单。" : "无法加载社区榜单。");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [filters, sort]);

  useEffect(() => playIntentStore.subscribe(() => {
    // The authoritative response invalidates cached community snapshots after
    // a successful optimistic vote. A no-op observer keeps this screen fresh.
    if (!playIntentStore.pendingCount()) void apiClient.community(sort, filters).then((result) => apply(result.data, false)).catch(() => undefined);
  }), [filters, sort]);

  const loadMore = async () => {
    if (!nextCursor) return;
    try {
      const result = await apiClient.community(sort, filters, nextCursor);
      apply(result.data, true);
    } catch (cause) {
      if (cause instanceof ApiError && cause.code === "cursor_stale") {
        const first = await apiClient.community(sort, filters);
        apply(first.data, false);
      } else {
        setError("无法加载更多条目。");
      }
    }
  };

  return (
    <section className="community-screen" aria-label="大家想玩">
      <div className="screen-head">
        <div>
          <h2>大家想玩</h2>
          {updatedAt !== null && <p>数据更新于 {formatAgo(updatedAt)}</p>}
        </div>
        <div className="seg" aria-label="社区排序">
          <button type="button" className="btn small" aria-pressed={sort === "trending"} onClick={() => setSort("trending")}>正在升温</button>
          <button type="button" className="btn small" aria-pressed={sort === "most_voted"} onClick={() => setSort("most_voted")}>最多人想玩</button>
        </div>
      </div>
      <div className="community-filters" aria-label="社区筛选">
        <label>
          <span>发售状态</span>
          <select value={releaseState} onChange={(event) => setReleaseState(event.target.value as CommunityReleaseState | "")}>
            <option value="">全部</option>
            <option value="upcoming">即将发售</option>
            <option value="coming_soon">敬请期待</option>
            <option value="released">已发售</option>
            <option value="retired">已下架</option>
            <option value="unknown">状态未知</option>
          </select>
        </label>
        <label>
          <span>平台</span>
          <select value={platform} onChange={(event) => setPlatform(event.target.value as CommunityPlatform | "")}>
            <option value="">全部</option>
            <option value="windows">Windows</option>
            <option value="macos">macOS</option>
            <option value="linux">Linux</option>
          </select>
        </label>
        <label>
          <span>人数</span>
          <input
            type="number"
            min={1}
            max={64}
            inputMode="numeric"
            value={partySize}
            onChange={(event) => setPartySize(event.target.value)}
          />
        </label>
        <label className="community-demo-filter">
          <input type="checkbox" checked={demoOnly} onChange={(event) => setDemoOnly(event.target.checked)} />
          <span>仅 Demo</span>
        </label>
      </div>
      {loading && <div className="skeleton" style={{ height: 220 }} />}
      {error && <p className="cal-note">{error}</p>}
      {!loading && !error && items.length === 0 && <p className="empty-state">还没有公开的想玩投票。</p>}
      <div className="community-list">
        {items.map((item) => (
          <article key={item.app_id} className="community-row">
            <button type="button" className="community-main" onClick={() => onOpenGame(item.app_id)}>
              <GameMedia coverUrl={item.cover_url} name={item.name} appId={item.app_id} compact />
              <span className="community-copy">
                <strong>{item.name}</strong>
                <span>{releaseStateLabel(item.release_state)} · {formatReleaseDate(item.release_date, item.release_date_raw, item.release_date_precision)}</span>
                <span className="community-trend">{sort === "trending" ? `近 7 天 ${formatCount(item.trending_count)} 人加入` : `${formatCount(item.play_intent.count)} 人想玩`}</span>
              </span>
            </button>
            <div className="community-social">
              <Facepile voters={item.play_intent.voters_preview ?? []} omittedCount={item.play_intent.omitted_count ?? 0} total={item.play_intent.count} />
              <VoteButton appId={item.app_id} intent={item.play_intent} />
            </div>
          </article>
        ))}
      </div>
      {nextCursor && <button type="button" className="btn" onClick={() => void loadMore()}>加载更多</button>}
    </section>
  );
}
