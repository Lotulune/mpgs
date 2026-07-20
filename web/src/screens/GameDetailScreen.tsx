// Game detail: multiplayer profile, availability, reviews/CCU, evidence, Steam link.

import { useEffect, useRef, useState } from "react";
import { ApiError } from "../api/client";
import type { EvidenceItem, GameDetail, PopularReview } from "../api/types";
import {
  dominantModeLabel,
  evidenceValueLabel,
  featureLabel,
  formatAgo,
  formatReleaseDate,
  formatCount,
  formatPercent,
  formatPrice,
  isStale,
  languageLabels,
  partyLabel,
  platformLabels,
  positiveRate,
  releaseStateLabel,
  sourceTypeLabel,
} from "../app/format";
import { apiClient } from "../app/runtime";
import { useTheme } from "../app/ThemeProvider";
import { GameMedia } from "./GameMedia";
import { VoteButton } from "./VoteButton";

function boolLabel(value: boolean | null): string {
  if (value === true) return "支持";
  if (value === false) return "不支持";
  return "未知";
}

function fallbackSummary(game: GameDetail): string {
  const mp = game.multiplayer;
  const capabilities = [
    mp.private_session === true ? "可创建私人房间" : null,
    mp.online_coop === true ? "支持在线合作" : null,
    mp.self_hosted_server === true ? "可自建服务器" : null,
  ].filter((value): value is string => value !== null);
  const party = partyLabel(mp.recommended_min, mp.recommended_max);
  const capabilityText = capabilities.length > 0 ? capabilities.join("，") : "联机能力仍待补充";
  const partyText =
    party === "人数未定" ? "推荐人数仍待补充" : `推荐 ${party}`;
  return `${dominantModeLabel(mp.dominant_mode)}玩法，${partyText}；${capabilityText}。`;
}

function reviewDate(timestampMs: number): string {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "short",
    day: "numeric",
  }).format(new Date(timestampMs));
}

function playtimeLabel(minutes: number | null): string | null {
  if (minutes === null) return null;
  const hours = minutes / 60;
  return `游玩 ${hours >= 100 ? Math.round(hours) : hours.toFixed(1)} 小时`;
}

function PopularReviewCard({ review }: { review: PopularReview }) {
  const [expanded, setExpanded] = useState(false);
  const isLong = review.text.length > 360;
  const playtime = playtimeLabel(review.playtime_forever_minutes);
  return (
    <article className="steam-review-card">
      <div className="steam-review-head">
        <span className={review.voted_up ? "chip ok" : "chip danger"}>
          {review.voted_up ? "👍 推荐" : "👎 不推荐"}
        </span>
        {review.author_profile_url ? (
          <a href={review.author_profile_url} target="_blank" rel="noreferrer noopener">
            {review.author_name || "Steam 玩家"} ↗
          </a>
        ) : (
          <strong>{review.author_name || "Steam 玩家"}</strong>
        )}
        <span className="steam-review-rank">热门 #{review.rank}</span>
      </div>
      <div className="steam-review-meta">
        <span>{reviewDate(review.created_at_ms)}</span>
        {playtime && <span>{playtime}</span>}
        <span>{formatCount(review.votes_up)} 人觉得有用</span>
        {review.written_during_early_access && <span>抢先体验期间撰写</span>}
      </div>
      <p className={expanded ? "steam-review-text expanded" : "steam-review-text"}>
        {review.text}
      </p>
      {isLong && (
        <button type="button" className="review-expand" onClick={() => setExpanded((value) => !value)}>
          {expanded ? "收起" : "展开全文"}
        </button>
      )}
    </article>
  );
}

interface DetailState {
  detail: GameDetail | null;
  evidence: EvidenceItem[];
  loading: boolean;
  error: ApiError | null;
  fromOfflineCache: boolean;
}

export function GameDetailScreen({ appId, onBack }: { appId: number; onBack: () => void }) {
  const [state, setState] = useState<DetailState>({
    detail: null,
    evidence: [],
    loading: true,
    error: null,
    fromOfflineCache: false,
  });
  const { fireAction } = useTheme();
  const steamBtnRef = useRef<HTMLAnchorElement>(null);

  useEffect(() => {
    let cancelled = false;
    setState((prev) => ({ ...prev, loading: true, error: null }));
    Promise.all([apiClient.game(appId), apiClient.evidence(appId).catch(() => null)])
      .then(([game, evidence]) => {
        if (cancelled) return;
        setState({
          detail: game.data,
          evidence: evidence?.data.items ?? [],
          loading: false,
          error: null,
          fromOfflineCache: game.fromOfflineCache,
        });
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        setState({
          detail: null,
          evidence: [],
          loading: false,
          error:
            error instanceof ApiError
              ? error
              : new ApiError({ code: "unknown", status: 0, message: String(error) }),
          fromOfflineCache: false,
        });
      });
    return () => {
      cancelled = true;
    };
  }, [appId]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onBack();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onBack]);

  if (state.loading) {
    return (
      <div className="detail" aria-busy="true">
        <div className="backbar">
          <button type="button" className="btn small" onClick={onBack}>
            ← 返回 (Esc)
          </button>
        </div>
        <div className="skeleton" style={{ height: 90 }} />
        <div className="detail-grid">
          <div className="skeleton" style={{ height: 220 }} />
          <div className="skeleton" style={{ height: 220 }} />
        </div>
      </div>
    );
  }

  if (state.error || !state.detail) {
    return (
      <div className="detail">
        <div className="backbar">
          <button type="button" className="btn small" onClick={onBack}>
            ← 返回 (Esc)
          </button>
        </div>
        <div className="state-box" role="alert">
          <span className="big">!</span>
          <span>
            {state.error?.code === "not_found"
              ? "没有找到这个游戏。"
              : `详情加载失败：${state.error?.message ?? "未知错误"}`}
          </span>
        </div>
      </div>
    );
  }

  const game = state.detail;
  const mp = game.multiplayer;
  const av = game.availability;
  const partySizeLabel = partyLabel(mp.recommended_min, mp.recommended_max);
  const shortDescription = game.short_description?.trim() || null;
  const price = av.is_free === true || (av.final_price_minor !== null && av.price_currency)
    ? formatPrice(av.final_price_minor, av.price_currency, av.is_free)
    : "待同步 Steam 国区价格";

  return (
    <div className="detail">
      <div className="backbar">
        <button type="button" className="btn small" onClick={onBack}>
          ← 返回 (Esc)
        </button>
        {state.fromOfflineCache && <span className="chip danger">离线快照</span>}
        <span className={isStale(game.data_updated_at_ms) ? "chip warn" : "chip"}>
          数据更新于 {formatAgo(game.data_updated_at_ms)}
        </span>
      </div>

      <div className="detail-hero">
        <div className="detail-cover">
          <GameMedia coverUrl={game.cover_url} name={game.name} appId={game.app_id} />
        </div>
        <div className="detail-hero-body">
          <div className="detail-head">
            <div>
              <h2>{game.name}</h2>
              <div className="card-meta" style={{ marginTop: 8 }}>
                <span className="chip accent">{dominantModeLabel(mp.dominant_mode)}</span>
                <span className="chip">{releaseStateLabel(game.release_state)}</span>
                <span className="chip">{formatReleaseDate(game.release_date, game.release_date_raw, game.release_date_precision)}</span>
                {av.has_demo && <span className="chip ok">有 Demo</span>}
              </div>
            </div>
            <div className="detail-actions">
              <VoteButton appId={game.app_id} intent={game.play_intent} size="large" />
              <a
                ref={steamBtnRef}
                className="btn primary"
                href={game.steam_url}
                target="_blank"
                rel="noreferrer noopener"
                onClick={() => fireAction("confirm", steamBtnRef.current)}
              >
                在 Steam 打开 ↗
              </a>
            </div>
          </div>
          <section className="detail-summary" aria-label={shortDescription ? "商店简介" : "联机速览"}>
            <h3>{shortDescription ? "商店简介" : "联机速览"}</h3>
            <p>{shortDescription ?? fallbackSummary(game)}</p>
            <span>{shortDescription ? "来源：Steam 商店" : "来源：已入库联机资料"}</span>
          </section>
        </div>
      </div>

      <div className="detail-grid">
        <section className="panel">
          <h4>联机方式</h4>
          <dl className="kv">
            <dt>推荐人数</dt>
            <dd>
              {partySizeLabel}
              {partySizeLabel === "人数未定" && (
                <span
                  className="chip warn"
                  style={{ marginLeft: 8 }}
                  title="商店分类仅能确认多人，无法可靠得到小队人数区间"
                >
                  仅分类弱信号
                </span>
              )}
            </dd>
            <dt>私人房间</dt>
            <dd>{boolLabel(mp.private_session)}</dd>
            <dt>在线合作</dt>
            <dd>{boolLabel(mp.online_coop)}</dd>
            <dt>自建服务器</dt>
            <dd>{boolLabel(mp.self_hosted_server)}</dd>
            <dt>画像置信度</dt>
            <dd>
              {formatPercent(mp.profile_confidence)}
              {mp.profile_confidence !== null && mp.profile_confidence < 0.5 && (
                <span className="chip warn" style={{ marginLeft: 8 }}>
                  低置信
                </span>
              )}
            </dd>
          </dl>
        </section>

        <section className="panel">
          <h4>可用性</h4>
          <dl className="kv">
            <dt>平台</dt>
            <dd>{av.platforms.length > 0 ? platformLabels(av.platforms) : "待同步 Steam 商店资料"}</dd>
            <dt>语言</dt>
            <dd>{av.languages.length > 0 ? languageLabels(av.languages) : "待同步 Steam 商店资料"}</dd>
            <dt>单局时长</dt>
            <dd>
              {av.typical_session_minutes_min !== null && av.typical_session_minutes_max !== null
                ? `${av.typical_session_minutes_min}–${av.typical_session_minutes_max} 分钟`
                : "尚未录入"}
            </dd>
            <dt>价格</dt>
            <dd>{price}</dd>
          </dl>
        </section>

        <section className="panel">
          <h4>评价与活跃度</h4>
          <dl className="kv">
            <dt>累计评价</dt>
            <dd>{formatCount(game.reviews.total)}</dd>
            <dt>好评率</dt>
            <dd>{positiveRate(game.reviews.total, game.reviews.positive)}</dd>
            <dt>当前在线</dt>
            <dd>{game.latest_ccu !== null ? formatCount(game.latest_ccu) : "未知"}</dd>
          </dl>
        </section>

        <section className="panel">
          <h4>特征证据</h4>
          {state.evidence.length === 0 ? (
            <span style={{ fontSize: 13, color: "var(--ink-muted)" }}>
              暂无公开证据记录。
            </span>
          ) : (
            state.evidence.slice(0, 8).map((item) => (
              <div key={item.evidence_id} className="evidence-item">
                <span>
                  {featureLabel(item.feature)} = {evidenceValueLabel(item.value)}
                </span>
                <span className="src">
                  {sourceTypeLabel(item.source_type)} · 置信 {formatPercent(item.confidence)} ·{" "}
                  {formatAgo(item.observed_at_ms)}
                </span>
              </div>
            ))
          )}
        </section>

        <section className="panel review-panel">
          <div className="review-panel-title">
            <div>
              <h4>Steam 热门评价</h4>
              <span>简体中文 · 按 Steam 热门顺序 · 最多 10 条</span>
            </div>
          </div>
          {game.reviews.featured.length === 0 ? (
            <div className="review-empty">热门评价正文尚未同步。</div>
          ) : (
            <div className="steam-review-grid">
              {game.reviews.featured.map((review) => (
                <PopularReviewCard key={review.recommendation_id} review={review} />
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
