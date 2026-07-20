// Feed screen: section tabs live in the shell; this renders one section's list
// with page-based pagination and optional sort controls.

import { useEffect, useState } from "react";
import type { FeedSection, FeedSort, FeedSortOrder } from "../api/types";
import { FEED_SORT_OPTIONS } from "../api/types";
import { formatAgo, isStale, SECTION_META } from "../app/format";
import { feedbackQueue } from "../app/runtime";
import { defaultOrderForSort, useFeed } from "../app/useFeed";
import { GameCard } from "./GameCard";

function PaginationBar({
  page,
  totalPages,
  total,
  loading,
  onPage,
}: {
  page: number;
  totalPages: number;
  total: number;
  loading: boolean;
  onPage: (page: number) => void;
}) {
  if (totalPages <= 1) {
    return total > 0 ? (
      <div className="pagination" aria-label="分页">
        <span className="pagination-meta">共 {total} 款</span>
      </div>
    ) : null;
  }

  const windowSize = 5;
  let start = Math.max(1, page - Math.floor(windowSize / 2));
  let end = Math.min(totalPages, start + windowSize - 1);
  start = Math.max(1, end - windowSize + 1);
  const pages: number[] = [];
  for (let p = start; p <= end; p += 1) pages.push(p);

  return (
    <div className="pagination" aria-label="分页">
      <span className="pagination-meta">
        第 {page}/{totalPages} 页 · 共 {total} 款
      </span>
      <div className="pagination-controls">
        <button
          type="button"
          className="btn small"
          disabled={loading || page <= 1}
          onClick={() => onPage(page - 1)}
        >
          上一页
        </button>
        {start > 1 && (
          <>
            <button type="button" className="btn small ghost" disabled={loading} onClick={() => onPage(1)}>
              1
            </button>
            {start > 2 && <span className="pagination-ellipsis">…</span>}
          </>
        )}
        {pages.map((p) => (
          <button
            key={p}
            type="button"
            className={`btn small${p === page ? " primary" : " ghost"}`}
            disabled={loading || p === page}
            aria-current={p === page ? "page" : undefined}
            onClick={() => onPage(p)}
          >
            {p}
          </button>
        ))}
        {end < totalPages && (
          <>
            {end < totalPages - 1 && <span className="pagination-ellipsis">…</span>}
            <button
              type="button"
              className="btn small ghost"
              disabled={loading}
              onClick={() => onPage(totalPages)}
            >
              {totalPages}
            </button>
          </>
        )}
        <button
          type="button"
          className="btn small"
          disabled={loading || page >= totalPages}
          onClick={() => onPage(page + 1)}
        >
          下一页
        </button>
      </div>
    </div>
  );
}

function RankedFeedPanel({
  section,
  onOpenGame,
}: {
  section: FeedSection;
  onOpenGame: (appId: number) => void;
}) {
  const [sort, setSort] = useState<FeedSort>(
    section === "upcoming" ? "release_date" : "recommended",
  );
  const [order, setOrder] = useState<FeedSortOrder>(() =>
    defaultOrderForSort(section === "upcoming" ? "release_date" : "recommended", section),
  );
  const feed = useFeed(section, sort, order);
  const meta = SECTION_META[section];

  useEffect(() => {
    // Reset sort when switching tabs so "即将发售" defaults to release date.
    const nextSort: FeedSort = section === "upcoming" ? "release_date" : "recommended";
    setSort(nextSort);
    setOrder(defaultOrderForSort(nextSort, section));
  }, [section]);

  useEffect(() => feedbackQueue.subscribeRankingChanged(feed.reload), [feed.reload]);

  const onSelectSort = (next: FeedSort) => {
    setSort(next);
    setOrder(defaultOrderForSort(next, section));
  };

  const toggleOrder = () => {
    setOrder((current) => (current === "asc" ? "desc" : "asc"));
  };

  return (
    <section aria-label={meta.label}>
      <div className="statusline">
        <span>{meta.hint}</span>
        {section === "upcoming" && (
          <span className="chip">按推荐流展示；完整按日日历见「日历」页</span>
        )}
        {feed.dataUpdatedAtMs !== null && (
          <span className={isStale(feed.dataUpdatedAtMs) ? "chip warn" : "chip"}>
            数据更新于 {formatAgo(feed.dataUpdatedAtMs)}
          </span>
        )}
        {feed.fromOfflineCache && <span className="chip danger">离线快照</span>}
        {feed.algorithmVersion && <span className="chip">{feed.algorithmVersion}</span>}
      </div>

      <div className="feed-sortbar" role="toolbar" aria-label="排序">
        {FEED_SORT_OPTIONS.map((option) => (
          <button
            key={option.id}
            type="button"
            className={`btn small${sort === option.id ? " primary" : " ghost"}`}
            aria-pressed={sort === option.id}
            onClick={() => onSelectSort(option.id)}
          >
            {option.label}
          </button>
        ))}
        <button
          type="button"
          className="btn small ghost"
          onClick={toggleOrder}
          title={order === "asc" ? "升序" : "降序"}
          aria-label={order === "asc" ? "当前升序，点击切换为降序" : "当前降序，点击切换为升序"}
        >
          {order === "asc" ? "升序 ↑" : "降序 ↓"}
        </button>
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
          <span>
            {section === "upcoming"
              ? "暂无符合条件的即将发售多人候选。可在「日历」页查看发售窗口，或等待商店采集写入 coming soon 记录。"
              : "该分区暂时没有符合条件的候选。数据允许时会如实展示，不会伪造推荐。"}
          </span>
          <button type="button" className="btn" onClick={feed.reload}>
            刷新
          </button>
        </div>
      )}

      {feed.items.length > 0 && (
        <>
          <PaginationBar
            page={feed.page}
            totalPages={feed.totalPages}
            total={feed.total}
            loading={feed.loading}
            onPage={feed.goToPage}
          />
          <div className="feed-grid">
            {feed.items.map((item) => (
              <GameCard key={item.app_id} item={item} onOpen={onOpenGame} />
            ))}
          </div>
          <PaginationBar
            page={feed.page}
            totalPages={feed.totalPages}
            total={feed.total}
            loading={feed.loading}
            onPage={(p) => {
              feed.goToPage(p);
              window.scrollTo({ top: 0, behavior: "smooth" });
            }}
          />
        </>
      )}
    </section>
  );
}

export function FeedScreen({
  section,
  onOpenGame,
}: {
  section: FeedSection;
  onOpenGame: (appId: number) => void;
}) {
  return <RankedFeedPanel section={section} onOpenGame={onOpenGame} />;
}
