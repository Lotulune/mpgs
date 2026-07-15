// Game detail: multiplayer profile, availability, reviews/CCU, evidence, Steam link.

import { useEffect, useRef, useState } from "react";
import { ApiError } from "../api/client";
import type { EvidenceItem, GameDetail } from "../api/types";
import {
  dominantModeLabel,
  evidenceValueLabel,
  featureLabel,
  formatAgo,
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
import { VoteButton } from "./VoteButton";

function boolLabel(value: boolean | null): string {
  if (value === true) return "支持";
  if (value === false) return "不支持";
  return "未知";
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

      <div className="detail-head">
        <div>
          <h2>{game.name}</h2>
          <div className="card-meta" style={{ marginTop: 8 }}>
            <span className="chip accent">{dominantModeLabel(mp.dominant_mode)}</span>
            <span className="chip">{releaseStateLabel(game.release_state)}</span>
            <span className="chip">{game.release_date ?? "发售日期未知"}</span>
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

      <div className="detail-grid">
        <section className="panel">
          <h4>联机方式</h4>
          <dl className="kv">
            <dt>推荐人数</dt>
            <dd>{partyLabel(mp.recommended_min, mp.recommended_max)}</dd>
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
            <dd>{platformLabels(av.platforms)}</dd>
            <dt>语言</dt>
            <dd>{languageLabels(av.languages)}</dd>
            <dt>单局时长</dt>
            <dd>
              {av.typical_session_minutes_min !== null && av.typical_session_minutes_max !== null
                ? `${av.typical_session_minutes_min}–${av.typical_session_minutes_max} 分钟`
                : "未知"}
            </dd>
            <dt>价格</dt>
            <dd>{formatPrice(av.final_price_minor, av.price_currency, av.is_free)}</dd>
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
      </div>
    </div>
  );
}
