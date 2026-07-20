// Release calendar: recent/upcoming periods, early-data context and app-type filters.

import { useEffect, useMemo, useState } from "react";
import { ApiError } from "../api/client";
import type { CalendarItem, CalendarPeriod, CalendarResponse } from "../api/types";
import { apiClient } from "../app/runtime";
import {
  appTypeLabel,
  confidenceLabel,
  defaultWindow,
  earlyDataLabel,
  groupByMonth,
  precisionLabel,
  recentWindow,
} from "../app/calendar";
import { formatAgo, formatReleaseDate, isStale, releaseStateLabel } from "../app/format";

interface CalendarState {
  data: CalendarResponse | null;
  loading: boolean;
  error: ApiError | null;
  fromOfflineCache: boolean;
}

type CalendarTypeFilter = "all" | "game" | "demo" | "playtest";

const TYPE_FILTER_LABELS: Record<CalendarTypeFilter, string> = {
  all: "全部",
  game: "正式游戏",
  demo: "Demo",
  playtest: "Playtest",
};

function matchesType(item: CalendarItem, typeFilter: CalendarTypeFilter): boolean {
  if (typeFilter === "all") return true;
  if (typeFilter === "game") return !["demo", "playtest"].includes(item.app_type);
  return item.app_type === typeFilter;
}

function CalendarRow({
  item,
  onOpenGame,
}: {
  item: CalendarItem;
  onOpenGame: (appId: number) => void;
}) {
  const earlyLabel = earlyDataLabel(item.early_data, item.review_total);
  return (
    <button type="button" className="cal-row" onClick={() => onOpenGame(item.app_id)}>
      <span className="cal-day">{formatReleaseDate(item.release_date, item.release_date_raw, item.release_date_precision)}</span>
      <span className="cal-name">{item.canonical_name}</span>
      <span className="cal-tags">
        <span className="chip accent">{appTypeLabel(item.app_type)}</span>
        <span className="chip">{releaseStateLabel(item.release_state)}</span>
        {item.is_early_access && <span className="chip warn">抢先体验</span>}
        {earlyLabel && <span className="chip warn">{earlyLabel}</span>}
        {precisionLabel(item.release_date_precision) && (
          <span className="chip">{precisionLabel(item.release_date_precision)}</span>
        )}
        <span
          className={
            item.current_data_confidence !== null && item.current_data_confidence < 0.5
              ? "chip warn"
              : "chip"
          }
        >
          {confidenceLabel(item.current_data_confidence)}
        </span>
        <span className="cal-source">
          来源更新于 {formatAgo(item.source_modified_at_ms ?? item.updated_at_ms)}
        </span>
      </span>
    </button>
  );
}

export function CalendarScreen({ onOpenGame }: { onOpenGame: (appId: number) => void }) {
  const [period, setPeriod] = useState<CalendarPeriod>("upcoming");
  const [typeFilter, setTypeFilter] = useState<CalendarTypeFilter>("all");
  const [state, setState] = useState<CalendarState>({
    data: null,
    loading: true,
    error: null,
    fromOfflineCache: false,
  });

  useEffect(() => {
    let cancelled = false;
    const { from, to } =
      period === "upcoming" ? defaultWindow(Date.now(), 6) : recentWindow(Date.now(), 6);
    setState((prev) => ({ ...prev, loading: true, error: null }));
    apiClient
      .calendar(from, to, period)
      .then((result) => {
        if (cancelled) return;
        setState({
          data: result.data,
          loading: false,
          error: null,
          fromOfflineCache: result.fromOfflineCache,
        });
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        setState({
          data: null,
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
  }, [period]);

  const dated = useMemo(
    () =>
      (state.data?.dated_items ?? []).filter((item) => matchesType(item, typeFilter)),
    [state.data, typeFilter],
  );
  const months = useMemo(() => groupByMonth(dated), [dated]);
  const undated = (state.data?.undated_items ?? []).filter((item) =>
    matchesType(item, typeFilter),
  );

  return (
    <section aria-label="发售日历">
      <div className="statusline">
        <div className="seg" aria-label="日历时间范围">
          <button
            type="button"
            className="btn small"
            aria-pressed={period === "upcoming"}
            onClick={() => setPeriod("upcoming")}
          >
            即将发售
          </button>
          <button
            type="button"
            className="btn small"
            aria-pressed={period === "recent"}
            onClick={() => setPeriod("recent")}
          >
            近期发售
          </button>
        </div>
        <div className="seg" aria-label="条目类型筛选">
          {(Object.keys(TYPE_FILTER_LABELS) as CalendarTypeFilter[]).map((type) => (
            <button
              key={type}
              type="button"
              className="btn small"
              aria-pressed={typeFilter === type}
              onClick={() => setTypeFilter(type)}
            >
              {TYPE_FILTER_LABELS[type]}
            </button>
          ))}
        </div>
        {state.data && (
          <span className={isStale(state.data.data_updated_at_ms) ? "chip warn" : "chip"}>
            数据更新于 {formatAgo(state.data.data_updated_at_ms)}
          </span>
        )}
        {state.fromOfflineCache && <span className="chip danger">离线快照</span>}
      </div>

      {state.loading && (
        <div className="cal-list" aria-busy="true">
          {Array.from({ length: 5 }, (_, i) => (
            <div key={i} className="skeleton" style={{ height: 48 }} />
          ))}
        </div>
      )}

      {!state.loading && state.error && (
        <div className="state-box" role="alert">
          <span className="big">{state.error.offline ? "⌁" : "!"}</span>
          <span>
            {state.error.offline ? "网络不可用，且没有可用的日历缓存。" : `加载失败：${state.error.message}`}
          </span>
        </div>
      )}

      {!state.loading && !state.error && months.length === 0 && undated.length === 0 && (
        <div className="state-box">
          <span className="big">∅</span>
          <span>当前时间与类型筛选下没有已知的发售条目。</span>
        </div>
      )}

      {months.map((group) => (
        <div key={group.key} className="cal-month">
          <h3 className="cal-month-title">{group.label}</h3>
          <div className="cal-list">
            {group.items.map((item) => (
              <CalendarRow key={item.app_id} item={item} onOpenGame={onOpenGame} />
            ))}
          </div>
        </div>
      ))}

      {undated.length > 0 && (
        <div className="cal-month">
          <h3 className="cal-month-title">日期未定</h3>
          <p className="cal-note">这些游戏尚未公布确切日期，不会被伪造成具体日期。</p>
          <div className="cal-list">
            {undated.map((item) => (
              <CalendarRow key={item.app_id} item={item} onOpenGame={onOpenGame} />
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
