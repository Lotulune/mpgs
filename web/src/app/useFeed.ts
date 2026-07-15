// Feed loading hook: first page via ETag/offline cache, cursor pagination,
// stale/offline surfacing. State is kept per section and reset on section change.

import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "../api/client";
import type { FeedItem, FeedResponse, FeedSection } from "../api/types";
import { apiClient } from "./runtime";

export interface FeedState {
  items: FeedItem[];
  loading: boolean;
  loadingMore: boolean;
  error: ApiError | null;
  nextCursor: string | null;
  dataUpdatedAtMs: number | null;
  fromOfflineCache: boolean;
  algorithmVersion: string | null;
}

const INITIAL: FeedState = {
  items: [],
  loading: true,
  loadingMore: false,
  error: null,
  nextCursor: null,
  dataUpdatedAtMs: null,
  fromOfflineCache: false,
  algorithmVersion: null,
};

function toApiError(error: unknown): ApiError {
  return error instanceof ApiError
    ? error
    : new ApiError({
        code: "unknown",
        status: 0,
        message: error instanceof Error ? error.message : "unknown error",
      });
}

export function useFeed(section: FeedSection): FeedState & {
  reload: () => void;
  loadMore: () => void;
} {
  const [state, setState] = useState<FeedState>(INITIAL);
  const generation = useRef(0);
  const stateRef = useRef(state);
  stateRef.current = state;

  const load = useCallback(() => {
    const gen = generation.current + 1;
    generation.current = gen;
    setState({ ...INITIAL, loading: true });
    apiClient
      .feed(section, { limit: 20 })
      .then((result) => {
        if (generation.current !== gen) return;
        const data: FeedResponse = result.data;
        setState({
          items: data.items,
          loading: false,
          loadingMore: false,
          error: null,
          nextCursor: data.next_cursor,
          dataUpdatedAtMs: data.data_updated_at_ms,
          fromOfflineCache: result.fromOfflineCache,
          algorithmVersion: data.algorithm_version,
        });
      })
      .catch((error: unknown) => {
        if (generation.current !== gen) return;
        setState((prev) => ({ ...prev, loading: false, error: toApiError(error) }));
      });
  }, [section]);

  useEffect(() => {
    load();
  }, [load]);

  const loadMore = useCallback(() => {
    const current = stateRef.current;
    if (!current.nextCursor || current.loadingMore) return;
    const cursor = current.nextCursor;
    const gen = generation.current;
    setState((prev) => ({ ...prev, loadingMore: true }));
    apiClient
      .feed(section, { limit: 20, cursor })
      .then((result) => {
        if (generation.current !== gen) return;
        setState((cur) => ({
          ...cur,
          items: [...cur.items, ...result.data.items],
          nextCursor: result.data.next_cursor,
          loadingMore: false,
        }));
      })
      .catch((error: unknown) => {
        if (generation.current !== gen) return;
        // A stale cursor means the snapshot moved; restart from a fresh first page.
        if (error instanceof ApiError && error.code === "cursor_stale") {
          load();
          return;
        }
        setState((cur) => ({ ...cur, loadingMore: false }));
      });
  }, [section, load]);

  return { ...state, reload: load, loadMore };
}
