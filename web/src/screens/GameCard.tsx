// One recommendation card: large cover, release/review stats, reasons, feedback.
// Information priority is fixed: cover > title+score > meta chips > want-to-play
// > reasons > cautions > feedback actions.

import { useEffect, useRef, useState } from "react";
import type { FeedItem, FeedbackType } from "../api/types";
import { requestAccountSignIn } from "../app/auth";
import { apiClient, feedbackQueue } from "../app/runtime";
import { useTheme } from "../app/ThemeProvider";
import { useToast } from "../app/ToastProvider";
import {
  dominantModeLabel,
  FEEDBACK_LABELS,
  formatCount,
  formatReleaseDate,
  hasConcretePartySize,
  partyLabel,
  positiveRate,
} from "../app/format";
import type { PendingFeedback } from "../api/feedbackQueue";
import { Button } from "../components/Button";
import { Chip } from "../components/Chip";
import { ScoreBadge } from "../components/ScoreBadge";
import { VoteButton } from "../components/VoteButton";
import { GameMedia } from "../components/GameMedia";

const QUICK_ACTIONS: { type: FeedbackType; label: string }[] = [
  { type: "like", label: "喜欢" },
  { type: "played", label: "玩过" },
  { type: "not_interested", label: "不感兴趣" },
];

export function GameCard({
  item,
  onOpen,
}: {
  item: FeedItem;
  onOpen: (appId: number) => void;
}) {
  const { fireAction } = useTheme();
  const toast = useToast();
  const cardRef = useRef<HTMLElement>(null);
  const [active, setActive] = useState<PendingFeedback | null>(
    () => feedbackQueue.activeByApp().get(item.app_id) ?? null,
  );

  useEffect(() => {
    return feedbackQueue.subscribe(() => {
      setActive(feedbackQueue.activeByApp().get(item.app_id) ?? null);
    });
  }, [item.app_id]);

  const submit = (type: FeedbackType, target: Element | null) => {
    if (!apiClient.isAccountAuthenticated()) {
      requestAccountSignIn();
      return;
    }
    const entry = feedbackQueue.submit(item.app_id, type);
    fireAction(type === "like" ? "like" : type === "not_interested" ? "dismiss" : "confirm", target);
    toast.show(`已记录「${FEEDBACK_LABELS[type] ?? type}」`, {
      label: "撤销",
      run: () => {
        void feedbackQueue.undo(entry.localId).catch(() => {
          toast.show("撤销失败，请稍后再试");
        });
      },
    });
  };

  const ccu = item.typical_ccu_7d ?? item.latest_ccu;
  const releaseLabel = formatReleaseDate(
    item.release_date,
    item.release_date_raw,
    item.release_date_precision,
  );
  const hasReviews = typeof item.total_reviews === "number" && item.total_reviews > 0;
  const reviewLabel = hasReviews
    ? `${positiveRate(item.total_reviews, item.total_positive ?? null)} · ${formatCount(item.total_reviews)} 评`
    : null;
  const hasCcu = typeof ccu === "number" && ccu > 0;
  const mode = item.multiplayer?.dominant_mode ?? null;
  const partyMin = item.party?.recommended_min ?? null;
  const partyMax = item.party?.recommended_max ?? null;
  const showParty = hasConcretePartySize(partyMin, partyMax);

  return (
    <article
      ref={cardRef}
      className="card card-with-cover"
      tabIndex={0}
      role="button"
      aria-label={`查看 ${item.name} 详情`}
      onClick={() => onOpen(item.app_id)}
      onKeyDown={(event) => {
        if (event.target !== event.currentTarget) return;
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onOpen(item.app_id);
        }
      }}
    >
      <GameMedia coverUrl={item.cover_url} name={item.name} appId={item.app_id} />
      <div className="card-body">
        <div className="card-title">
          <h3>{item.name}</h3>
          <ScoreBadge score={item.score} />
        </div>
        <div className="card-meta">
          <Chip tone="accent">{dominantModeLabel(mode)}</Chip>
          {showParty && <Chip>{partyLabel(partyMin, partyMax)}</Chip>}
          {releaseLabel !== "日期未定" && <Chip>{releaseLabel}</Chip>}
          {reviewLabel && <Chip>{reviewLabel}</Chip>}
          {hasCcu && <Chip>约 {formatCount(ccu)} 在线</Chip>}
          {item.confidence < 0.5 && <Chip tone="warn">低置信数据</Chip>}
        </div>
        <div
          className="card-vote"
          onClick={(event) => event.stopPropagation()}
          onKeyDown={(event) => event.stopPropagation()}
        >
          <VoteButton appId={item.app_id} intent={item.play_intent} />
          <span className="card-vote-hint">想玩的朋友越多，推荐越靠前</span>
        </div>
        {item.ai_reasons && item.ai_reasons.length > 0 && (
          <div className="reason-block">
            <span className="reason-tag">AI 分析</span>
            <ul className="reason-list">
              {item.ai_reasons.slice(0, 3).map((reason) => (
                <li key={`ai-${reason}`}>{reason}</li>
              ))}
            </ul>
          </div>
        )}
        {item.reasons && item.reasons.length > 0 ? (
          <ul className="reason-list">
            {item.reasons.slice(0, 3).map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
          </ul>
        ) : (
          <p className="card-empty-hint">
            {[
              releaseLabel !== "日期未定" ? `发售 ${releaseLabel}` : null,
              reviewLabel,
              hasCcu ? `约 ${formatCount(ccu)} 在线` : null,
              mode ? dominantModeLabel(mode) : "联机画像未校准",
            ]
              .filter(Boolean)
              .join(" · ")}
          </p>
        )}
        {item.cautions && item.cautions.length > 0 && (
          <ul className="caution-list">
            {item.cautions.slice(0, 2).map((caution) => (
              <li key={caution}>{caution}</li>
            ))}
          </ul>
        )}
        <div
          className="card-actions"
          onClick={(event) => event.stopPropagation()}
          onKeyDown={(event) => event.stopPropagation()}
        >
          {active && !active.cancelled && !active.undone ? (
            <span className="feedback-state">
              已反馈：{FEEDBACK_LABELS[active.type] ?? active.type}
              {active.feedbackId === null && <Chip tone="warn">待同步</Chip>}
              <Button
                size="small"
                variant="ghost"
                onClick={(event) => {
                  const entry = active;
                  fireAction("dismiss", event.currentTarget);
                  void feedbackQueue.undo(entry.localId).catch(() => {
                    toast.show("撤销失败，请稍后再试");
                  });
                }}
              >
                撤销
              </Button>
            </span>
          ) : (
            QUICK_ACTIONS.map((action) => (
              <Button
                key={action.type}
                size="small"
                variant="ghost"
                onClick={(event) => submit(action.type, event.currentTarget)}
              >
                {action.label}
              </Button>
            ))
          )}
        </div>
      </div>
    </article>
  );
}
