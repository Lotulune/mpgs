// One recommendation card: reasons, cautions, score, party size, feedback bar.

import { useEffect, useRef, useState } from "react";
import type { FeedItem, FeedbackType } from "../api/types";
import { feedbackQueue } from "../app/runtime";
import { useTheme } from "../app/ThemeProvider";
import { useToast } from "../app/ToastProvider";
import {
  dominantModeLabel,
  FEEDBACK_LABELS,
  formatPercent,
  partyLabel,
} from "../app/format";
import type { PendingFeedback } from "../api/feedbackQueue";
import { VoteButton } from "./VoteButton";

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

  return (
    <article
      ref={cardRef}
      className="card"
      tabIndex={0}
      role="button"
      aria-label={`查看 ${item.name} 详情`}
      onClick={() => onOpen(item.app_id)}
      onKeyDown={(event) => {
        if (event.key === "Enter") onOpen(item.app_id);
      }}
    >
      <div className="card-title">
        <h3>{item.name}</h3>
        <span className="score-badge" title="综合适配分">
          {formatPercent(item.score)}
        </span>
      </div>
      <div className="card-meta">
        <span className="chip accent">{dominantModeLabel(item.multiplayer.dominant_mode)}</span>
        <span className="chip">{partyLabel(item.party.recommended_min, item.party.recommended_max)}</span>
        {item.confidence < 0.5 && <span className="chip warn">低置信数据</span>}
        <span className="card-meta-spacer" />
        <span onClick={(event) => event.stopPropagation()}>
          <VoteButton appId={item.app_id} intent={item.play_intent} />
        </span>
      </div>
      {item.ai_reasons && item.ai_reasons.length > 0 && (
        <ul className="reason-list">
          {item.ai_reasons.slice(0, 3).map((reason) => (
            <li key={`ai-${reason}`}>{reason}</li>
          ))}
        </ul>
      )}
      {item.reasons.length > 0 && (
        <ul className="reason-list">
          {item.reasons.slice(0, 3).map((reason) => (
            <li key={reason}>{reason}</li>
          ))}
        </ul>
      )}
      {item.cautions.length > 0 && (
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
            {active.feedbackId === null && <span className="chip warn">待同步</span>}
            <button
              type="button"
              className="btn small ghost"
              onClick={(event) => {
                const entry = active;
                fireAction("dismiss", event.currentTarget);
                void feedbackQueue.undo(entry.localId).catch(() => {
                    toast.show("撤销失败，请稍后再试");
                });
              }}
            >
              撤销
            </button>
          </span>
        ) : (
          QUICK_ACTIONS.map((action) => (
            <button
              key={action.type}
              type="button"
              className="btn small ghost"
              onClick={(event) => submit(action.type, event.currentTarget)}
            >
              {action.label}
            </button>
          ))
        )}
      </div>
    </article>
  );
}
