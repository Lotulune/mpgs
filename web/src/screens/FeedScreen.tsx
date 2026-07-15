// Feed screen: section tabs live in the shell; this renders one section's list
// with loading / empty / error / stale / offline states.

import type { FeedSection } from "../api/types";
import { formatAgo, isStale, SECTION_META } from "../app/format";
import { useFeed } from "../app/useFeed";
import { GameCard } from "./GameCard";

export function FeedScreen({
  section,
  onOpenGame,
}: {
  section: FeedSection;
  onOpenGame: (appId: number) => void;
}) {
  const feed = useFeed(section);
  const meta = SECTION_META[section];

  return (
    <section aria-label={meta.label}>
      <div className="statusline">
        <span>{meta.hint}</span>
        {feed.dataUpdatedAtMs !== null && (
          <span className={isStale(feed.dataUpdatedAtMs) ? "chip warn" : "chip"}>
            数据更新于 {formatAgo(feed.dataUpdatedAtMs)}
          </span>
        )}
        {feed.fromOfflineCache && <span className="chip danger">离线快照</span>}
        {feed.algorithmVersion && <span className="chip">{feed.algorithmVersion}</span>}
      </div>

      {feed.loading && (
        <div className="feed-grid" aria-busy="true">
          {Array.from({ length: 6 }, (_, i) => (
            <div key={i} className="skeleton" />
          ))}
        </div>
      )}

      {!feed.loading && feed.error && (
        <div className="state-box" role="alert">
          <span className="big">{feed.error.offline ? "⌁" : "!"}</span>
          <span>
            {feed.error.offline
              ? "网络不可用，且本地没有可用的缓存快照。"
              : `加载失败：${feed.error.message}`}
          </span>
          {feed.error.requestId && (
            <span style={{ fontSize: 11, opacity: 0.6 }}>request_id: {feed.error.requestId}</span>
          )}
          <button type="button" className="btn" onClick={feed.reload}>
            重试
          </button>
        </div>
      )}

      {!feed.loading && !feed.error && feed.items.length === 0 && (
        <div className="state-box">
          <span className="big">∅</span>
          <span>该分区暂时没有符合条件的候选。数据允许时会如实展示，不会伪造推荐。</span>
          <button type="button" className="btn" onClick={feed.reload}>
            刷新
          </button>
        </div>
      )}

      {feed.items.length > 0 && (
        <>
          <div className="feed-grid">
            {feed.items.map((item) => (
              <GameCard key={item.app_id} item={item} onOpen={onOpenGame} />
            ))}
          </div>
          {feed.nextCursor && (
            <div className="load-more">
              <button
                type="button"
                className="btn"
                disabled={feed.loadingMore}
                onClick={feed.loadMore}
              >
                {feed.loadingMore ? (
                  <>
                    <span className="spin" /> 加载中
                  </>
                ) : (
                  "加载更多"
                )}
              </button>
            </div>
          )}
        </>
      )}
    </section>
  );
}
