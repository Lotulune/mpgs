import { useMemo, useState, type FormEvent } from "react";
import type {
  AiAssessment,
  AiRecommendationMessage,
  AiRecommendationRequest,
  AiRecommendationResponse,
  AiRecommendedGame,
  GameCard,
} from "../../types";
import { getDisplayedGameScore } from "../../features/library/gameScoreDisplay";

export type AiConversationSummary = {
  id: number;
  title: string;
  messageCount: number;
  updatedAt: number;
};

export const INITIAL_AI_MESSAGES: AiRecommendationMessage[] = [
  {
    role: "assistant",
    content:
      "告诉我人数、联机方式、氛围、想排除的类型，我会只从已入库且已发售的游戏里找。",
  },
];

export function AiAssistantPage({
  activeConversationId,
  games,
  assessment,
  conversations,
  messages,
  recommendation,
  selectedGame,
  isBusy,
  onAssess,
  onMessagesChange,
  onNewConversation,
  onOpen,
  onRecommend,
  onRecommendationChange,
  onSelectConversation,
}: {
  activeConversationId: number;
  games: GameCard[];
  assessment: AiAssessment | null;
  conversations: AiConversationSummary[];
  messages: AiRecommendationMessage[];
  recommendation: AiRecommendationResponse | null;
  selectedGame: GameCard | null;
  isBusy: boolean;
  onAssess: (game: GameCard) => void;
  onMessagesChange: (messages: AiRecommendationMessage[]) => void;
  onNewConversation: () => void;
  onOpen: (game: GameCard) => void;
  onRecommend: (request: AiRecommendationRequest) => Promise<AiRecommendationResponse>;
  onRecommendationChange: (recommendation: AiRecommendationResponse | null) => void;
  onSelectConversation: (id: number) => void;
}) {
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isHistoryOpen, setIsHistoryOpen] = useState(false);
  const [isThinking, setIsThinking] = useState(false);
  const hasRecommendation = recommendation !== null;
  const displayedItems = recommendation?.items ?? fallbackItems(games);
  const historyLabel = useMemo(
    () => `${Math.max(0, messages.filter((message) => message.role === "user").length)} 条`,
    [messages],
  );
  const sortedConversations = useMemo(
    () => [...conversations].sort((left, right) => right.updatedAt - left.updatedAt),
    [conversations],
  );

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const prompt = draft.trim();
    if (!prompt || isBusy) {
      return;
    }

    const contextMessages = messages.slice(-6);
    const userMessage: AiRecommendationMessage = { role: "user", content: prompt };
    const nextMessages = [...messages, userMessage];
    setDraft("");
    setError(null);
    onMessagesChange(nextMessages);
    setIsThinking(true);

    try {
      const response = await onRecommend({
        prompt,
        contextMessages,
        limit: 5,
      });
      onRecommendationChange(response);
      onMessagesChange([
        ...nextMessages,
        {
          role: "assistant",
          content: response.reply,
        },
      ]);
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      setError(message);
      onMessagesChange([
        ...nextMessages,
        { role: "assistant", content: `推荐失败：${message}` },
      ]);
    } finally {
      setIsThinking(false);
    }
  }

  return (
    <section className="ai-page">
      <div className="assistant-head">
        <h2>
          AI 智能推荐助手 <em>Beta</em>
        </h2>
        <div className="assistant-session-meta">
          {recommendation && (
            <span title={recommendation.diagnostic ?? undefined}>
              {recommendation.llmUsed || recommendation.source === "hybrid"
                ? "LLM 已增强"
                : "规则匹配"}
            </span>
          )}
          <button
            className="assistant-session-button"
            disabled={isBusy}
            onClick={() => {
              if (isBusy) {
                return;
              }
              setDraft("");
              setError(null);
              setIsHistoryOpen(false);
              onNewConversation();
            }}
            type="button"
          >
            新对话
          </button>
          <div className="assistant-history-menu">
            <button
              aria-expanded={isHistoryOpen}
              className="assistant-session-button"
              disabled={isBusy}
              onClick={() => setIsHistoryOpen((current) => !current)}
              type="button"
            >
              对话历史
            </button>
            {isHistoryOpen && (
              <div className="assistant-history-popover">
                {sortedConversations.map((conversation) => (
                  <button
                    className={
                      conversation.id === activeConversationId
                        ? "assistant-history-item active"
                        : "assistant-history-item"
                    }
                    disabled={isBusy}
                    key={conversation.id}
                    onClick={() => {
                      if (isBusy) {
                        return;
                      }
                      setDraft("");
                      setError(null);
                      setIsHistoryOpen(false);
                      onSelectConversation(conversation.id);
                    }}
                    type="button"
                  >
                    <strong>继续对话 {conversation.title}</strong>
                    <small>{conversation.messageCount} 条需求</small>
                  </button>
                ))}
              </div>
            )}
          </div>
          <span className="assistant-current-count">当前会话 {historyLabel}</span>
        </div>
      </div>

      {recommendation?.diagnostic && (
        <p className="assistant-diagnostic">{recommendation.diagnostic}</p>
      )}

      <div className="chat-thread" aria-label="AI 推荐对话">
        {messages.map((message, index) => (
          <div className={`chat-bubble ${message.role === "user" ? "user" : "bot"}`} key={index}>
            {message.content}
          </div>
        ))}
        {assessment && (
          <div className="chat-bubble bot">
            单项评估：{assessment.summary}
          </div>
        )}
        {isThinking && (
          <div className="chat-bubble bot thinking" aria-live="polite">
            AI 正在思考
            <span className="thinking-dots" aria-hidden="true">
              <i />
              <i />
              <i />
            </span>
          </div>
        )}
        {error && <div className="chat-bubble bot">推荐失败：{error}</div>}
      </div>

      <div className="recommend-list">
        {displayedItems.map((item) => (
          <RecommendationRow
            isBusy={isBusy}
            item={item}
            key={item.game.appid}
            mode={hasRecommendation ? "match" : "score"}
            onAssess={onAssess}
            onOpen={onOpen}
            selectedGame={selectedGame}
          />
        ))}
      </div>

      {recommendation?.followUpQuestion && (
        <p className="assistant-follow-up">{recommendation.followUpQuestion}</p>
      )}

      <form className="chat-input" onSubmit={handleSubmit}>
        <input
          aria-label="推荐需求"
          disabled={isBusy}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="描述你想要的游戏，例如：本地合作、轻松、不要恐怖"
          value={draft}
        />
        <button
          aria-label="发送需求"
          disabled={isBusy || draft.trim().length === 0}
          type="submit"
        >
          <span aria-hidden="true">{isBusy ? "…" : "➤"}</span>
          <span className="sr-only">发送需求</span>
        </button>
      </form>
    </section>
  );
}

function RecommendationRow({
  item,
  mode,
  selectedGame,
  isBusy,
  onAssess,
  onOpen,
}: {
  item: AiRecommendedGame;
  mode: "match" | "score";
  selectedGame: GameCard | null;
  isBusy: boolean;
  onAssess: (game: GameCard) => void;
  onOpen: (game: GameCard) => void;
}) {
  const scoreDisplay = getDisplayedGameScore(item.game);
  const shownScore = mode === "match" ? item.matchScore : scoreDisplay.value;
  const scoreLabel =
    mode === "match" ? (item.exactMatch ? "匹配度" : "近似匹配") : scoreDisplay.label;
  const scoreValue =
    mode === "match" ? `${Math.round(shownScore)}%` : `${Math.round(shownScore)}`;

  return (
    <article className="recommend-row">
      <img src={item.game.capsuleUrl} alt="" />
      <div>
        <h3>{item.game.name}</h3>
        <p>
          {item.game.tags.join(" · ")} · {formatPct(item.game.positiveReviewPct)} 好评
        </p>
        <span>{item.reason}</span>
        <div className="recommend-traits">
          {item.matchedTraits.length > 0 && (
            <small>命中：{item.matchedTraits.join("、")}</small>
          )}
          {item.missingTraits.length > 0 && (
            <small>缺口：{item.missingTraits.join("、")}</small>
          )}
          {item.caveats.slice(0, 2).map((caveat) => (
            <small key={caveat}>风险：{caveat}</small>
          ))}
        </div>
      </div>
      <strong>
        {scoreValue}
        <small>{scoreLabel}</small>
      </strong>
      <div className="recommend-actions">
        <button type="button" onClick={() => onOpen(item.game)}>
          详情
        </button>
        <button type="button" disabled={isBusy} onClick={() => onAssess(item.game)}>
          {isBusy && selectedGame?.appid === item.game.appid ? "评估中" : "评估"}
        </button>
      </div>
    </article>
  );
}

function fallbackItems(games: GameCard[]): AiRecommendedGame[] {
  return games.map((game) => {
    const scoreDisplay = getDisplayedGameScore(game);
    return {
      game,
      matchScore: scoreDisplay.value,
      reason: game.aiSummary,
      matchedTraits: game.multiplayerModes.slice(0, 1),
      missingTraits: [],
      caveats: ["先描述需求，可获得更精确的匹配理由"],
      exactMatch: true,
    };
  });
}

function formatPct(value?: number | null) {
  return typeof value === "number" ? `${Math.round(value)}%` : "—";
}
