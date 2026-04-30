import type { GameAnalysisReport } from "../../types";

export function GameAnalysisPanel({
  report,
  loading,
  error,
  expanded,
  onRefresh,
  onToggleExpanded,
}: {
  report: GameAnalysisReport | null;
  loading: boolean;
  error: string | null;
  expanded: boolean;
  onRefresh: () => void;
  onToggleExpanded: () => void;
}) {
  if (!report) {
    if (loading) {
      return (
        <div className="detail-content-panel ai-analysis-report ai-analysis-state">
          <h3>AI 详细评估</h3>
          <p>正在生成这款游戏的详细分析，请稍等片刻。</p>
        </div>
      );
    }

    if (error) {
      return (
        <div className="detail-content-panel ai-analysis-report ai-analysis-state">
          <h3>AI 详细评估</h3>
          <p>{error}</p>
          <button className="gold-button" type="button" onClick={onRefresh}>
            重试生成
          </button>
        </div>
      );
    }

    return (
      <div className="detail-content-panel ai-analysis-report ai-analysis-state">
        <h3>AI 详细评估</h3>
        <p>当前还没有可展示的分析结果。</p>
      </div>
    );
  }

  const expandedRegionId = `game-analysis-expanded-${report.appid}`;

  return (
    <div className="detail-content-panel ai-analysis-report">
      <div className="analysis-summary-card">
        <div className="analysis-summary-head">
          <div className="analysis-summary-copy">
            <h3>AI 详细评估</h3>
            <p>{report.overview}</p>
          </div>
          <div className="analysis-score-card">
            <strong>{Math.round(report.overallScore)}</strong>
            <span>综合推荐值</span>
          </div>
        </div>

        <div className="analysis-summary-badges">
          <span className="analysis-badge analysis-badge-source">
            {formatSource(report.source)}
          </span>
          <span className="analysis-badge analysis-badge-confidence">
            {formatConfidence(report.confidence)}
          </span>
          <span className="analysis-badge">更新于 {formatGeneratedAt(report.generatedAt)}</span>
        </div>

        {error ? <p className="analysis-inline-alert">刷新失败：{error}</p> : null}

        <div className="analysis-dimension-grid">
          {report.dimensionScores.map((dimension) => (
            <article className="analysis-dimension-card" key={dimension.key}>
              <div className="analysis-dimension-head">
                <span>{dimension.label}</span>
                <strong>{Math.round(dimension.score)}</strong>
              </div>
              <i className="analysis-dimension-track">
                <b style={{ width: `${Math.round(dimension.score)}%` }} />
              </i>
              <p>{dimension.reason}</p>
            </article>
          ))}
        </div>

        <div className="analysis-action-row">
          <button
            aria-controls={expandedRegionId}
            aria-expanded={expanded}
            className="muted-button"
            type="button"
            onClick={onToggleExpanded}
          >
            {expanded ? "收起完整报告" : "查看完整报告"}
          </button>
          <button className="muted-button" disabled={loading} type="button" onClick={onRefresh}>
            {loading ? "刷新中…" : "刷新分析"}
          </button>
          <span className="analysis-action-hint">
            {loading ? "正在更新分析结果…" : "可展开查看优势、风险与证据明细。"}
          </span>
        </div>
      </div>

      <div
        aria-label="完整分析报告"
        className="analysis-expanded-grid"
        hidden={!expanded}
        id={expandedRegionId}
        role="region"
      >
          <section>
            <h4>优势亮点</h4>
            <div className="analysis-point-list">
              {report.strengths.length > 0 ? (
                report.strengths.map((point, index) => (
                  <article className="analysis-point-card" key={`${point.title}-${index}`}>
                    <strong>{point.title}</strong>
                    <p>{point.reason}</p>
                  </article>
                ))
              ) : (
                <p className="analysis-empty-copy">当前没有提炼出明显优势。</p>
              )}
            </div>
          </section>

          <section>
            <h4>关注风险</h4>
            <div className="analysis-point-list">
              {report.risks.length > 0 ? (
                report.risks.map((point, index) => (
                  <article className="analysis-point-card" key={`${point.title}-${index}`}>
                    <strong>{point.title}</strong>
                    <p>{point.reason}</p>
                  </article>
                ))
              ) : (
                <p className="analysis-empty-copy">当前没有提炼出明显风险。</p>
              )}
            </div>
          </section>

          <section>
            <h4>结构化证据</h4>
            <div className="analysis-evidence-list">
              {report.evidence.length > 0 ? (
                report.evidence.map((item, index) => (
                  <article className="analysis-evidence-card" key={`${item.label}-${index}`}>
                    <div className="analysis-evidence-head">
                      <strong>{item.label}</strong>
                      <span>{item.value}</span>
                    </div>
                    <p>{item.interpretation}</p>
                  </article>
                ))
              ) : (
                <p className="analysis-empty-copy">当前没有结构化证据。</p>
              )}
            </div>
          </section>

          <section>
            <h4>玩家评价证据</h4>
            <div className="analysis-review-list">
              {report.reviewEvidence.length > 0 ? (
                report.reviewEvidence.map((item, index) => (
                  <article className="analysis-review-card" key={`${item.quote}-${index}`}>
                    <div className="analysis-review-head">
                      <strong
                        className={
                          item.stance === "strength"
                            ? "review-sentiment review-sentiment-positive"
                            : "review-sentiment review-sentiment-negative"
                        }
                      >
                        {item.stance === "strength" ? "正向证据" : "风险证据"}
                      </strong>
                      <span>{item.playtimeText}</span>
                    </div>
                    <blockquote>{item.quote}</blockquote>
                    <p>{item.interpretation}</p>
                  </article>
                ))
              ) : (
                <p className="analysis-empty-copy">当前没有可引用的评论证据。</p>
              )}
            </div>
          </section>
      </div>
    </div>
  );
}

function formatSource(source: GameAnalysisReport["source"]) {
  return source === "hybrid" ? "混合分析" : "规则分析";
}

function formatConfidence(confidence: GameAnalysisReport["confidence"]) {
  switch (confidence) {
    case "high":
      return "高置信";
    case "medium":
      return "中置信";
    case "low":
      return "低置信";
  }
}

function formatGeneratedAt(generatedAt: string) {
  const date = new Date(generatedAt);
  if (Number.isNaN(date.getTime())) {
    return generatedAt;
  }

  return date.toLocaleString("zh-CN", {
    hour12: false,
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
