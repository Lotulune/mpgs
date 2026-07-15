// Search: debounced name/FTS search via GET /v1/search. No online AI.

import { useEffect, useRef, useState } from "react";
import { ApiError } from "../api/client";
import type { SearchItem } from "../api/types";
import { apiClient } from "../app/runtime";
import { useDebouncedValue } from "../app/useDebouncedValue";
import { releaseStateLabel } from "../app/format";
import { RequestGeneration } from "../app/requestGeneration";

interface SearchState {
  items: SearchItem[];
  loading: boolean;
  error: ApiError | null;
  query: string;
}

const EMPTY: SearchState = { items: [], loading: false, error: null, query: "" };

export function SearchScreen({ onOpenGame }: { onOpenGame: (appId: number) => void }) {
  const [query, setQuery] = useState("");
  const debounced = useDebouncedValue(query.trim(), 300);
  const [state, setState] = useState<SearchState>(EMPTY);
  const inputRef = useRef<HTMLInputElement>(null);
  const generation = useRef(new RequestGeneration());

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (debounced.length === 0) {
      generation.current.invalidate();
      setState(EMPTY);
      return;
    }
    const gen = generation.current.next();
    setState((prev) => ({ ...prev, loading: true, error: null, query: debounced }));
    apiClient
      .search(debounced, 30)
      .then((response) => {
        if (!generation.current.isCurrent(gen)) return;
        setState({ items: response.items, loading: false, error: null, query: debounced });
      })
      .catch((error: unknown) => {
        if (!generation.current.isCurrent(gen)) return;
        setState({
          items: [],
          loading: false,
          error:
            error instanceof ApiError
              ? error
              : new ApiError({ code: "unknown", status: 0, message: String(error) }),
          query: debounced,
        });
      });
  }, [debounced]);

  const updateQuery = (nextQuery: string) => {
    setQuery(nextQuery);
    if (nextQuery.trim().length === 0) {
      generation.current.invalidate();
      setState(EMPTY);
    }
  };

  return (
    <section aria-label="搜索">
      <div className="search-bar">
        <input
          ref={inputRef}
          type="search"
          className="search-input"
          placeholder="搜索游戏名称…"
          value={query}
          onChange={(event) => updateQuery(event.target.value)}
          aria-label="搜索游戏名称"
        />
        {state.loading && <span className="spin" aria-hidden="true" />}
      </div>

      {query.trim().length === 0 && (
        <div className="state-box">
          <span className="big">⌕</span>
          <span>输入游戏名称开始搜索。语义检索（自然语言）在 AI 阶段接入。</span>
        </div>
      )}

      {state.error && (
        <div className="state-box" role="alert">
          <span className="big">!</span>
          <span>
            {state.error.offline ? "网络不可用，搜索需要联网。" : `搜索失败：${state.error.message}`}
          </span>
        </div>
      )}

      {!state.error && state.query.length > 0 && !state.loading && state.items.length === 0 && (
        <div className="state-box">
          <span className="big">∅</span>
          <span>没有匹配「{state.query}」的游戏。</span>
        </div>
      )}

      {state.items.length > 0 && (
        <ul className="search-results">
          {state.items.map((item) => (
            <li key={item.app_id}>
              <button type="button" className="search-row" onClick={() => onOpenGame(item.app_id)}>
                <span className="search-name">{item.name}</span>
                <span className="search-meta">
                  <span className="chip">{releaseStateLabel(item.release_state)}</span>
                  <span className="chip">{item.release_date ?? "发售日期未知"}</span>
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
