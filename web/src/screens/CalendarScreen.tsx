// Calendar: upcoming releases grouped by month, plus undated items. GET /v1/calendar.

import { useEffect, useMemo, useState } from "react";
import { ApiError } from "../api/client";
import type { CalendarItem, CalendarResponse } from "../api/types";
import { apiClient } from "../app/runtime";
import {
  dayLabel,
  defaultWindow,
  groupByMonth,
  precisionLabel,
} from "../app/calendar";
import { formatAgo, isStale, releaseStateLabel } from "../app/format";

interface CalendarState {
  data: CalendarResponse | null;
  loading: boolean;
  error: ApiError | null;
  fromOfflineCache: boolean;
}

function CalendarRow({
  item,
  onOpenGame,
}: {
  item: CalendarItem;
  onOpenGame: (appId: number) => void;
}) {
  return (
    <button type="button" className="cal-row" onClick={() => onOpenGame(item.app_id)}>
      <span className="cal-day">{dayLabel(item.release_date)}</span>
      <span className="cal-name">{item.canonical_name}</span>
      <span className="cal-tags">
        <span className="chip">{releaseStateLabel(item.release_state)}</span>
        {item.is_early_access && <span className="chip warn">抢先体验</span>}
        {precisionLabel(item.release_date_precision) && (
          <span className="chip">{precisionLabel(item.release_date_precision)}</span>
        )}
      </span>
    </button>
  );
}

export function CalendarScreen({ onOpenGame }: { onOpenGame: (appId: number) => void }) {
  const [state, setState] = useState<CalendarState>({
    data: null,
    loading: true,
    error: null,
    fromOfflineCache: false,
  });

  useEffect(() => {
    let cancelled = false;
    const { from, to } = defaultWindow(Date.now(), 6);
    setState((prev) => ({ ...prev, loading: true, error: null }));
    apiClient
      .calendar(from, to)
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
  }, []);

  const months = useMemo(() => groupByMonth(state.data?.dated_items ?? []), [state.data]);
  const undated = state.data?.undated_items ?? [];

  return (
    <section aria-label="发售日历">
      <div className="statusline">
        <span>未来半年的发售与 Demo</span>
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
          <span>当前窗口没有已知的发售条目。</span>
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
