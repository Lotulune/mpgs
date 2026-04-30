import { useEffect, useRef, useState } from "react";
import { generateGameAnalysis, getGameAnalysis } from "../../api/client";
import type { GameAnalysisReport, GameCard } from "../../types";

export function useGameAnalysis(game: GameCard) {
  const [report, setReport] = useState<GameAnalysisReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);
  const activeRequestIdRef = useRef(0);
  const mountedRef = useRef(true);
  const currentAppidRef = useRef(game.appid);

  currentAppidRef.current = game.appid;

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
      activeRequestIdRef.current += 1;
    };
  }, []);

  useEffect(() => {
    setExpanded(false);
    void loadLatest(game.appid);
  }, [game.appid]);

  async function loadLatest(appid: number) {
    const requestId = beginRequest();
    setReport(null);

    try {
      const cachedReport = await getGameAnalysis(appid);
      if (!isRequestCurrent(requestId, appid)) {
        return;
      }

      if (cachedReport) {
        setReport(cachedReport);
        setLoading(false);
        return;
      }

      const generatedReport = await generateGameAnalysis(appid, false);
      if (!isRequestCurrent(requestId, appid)) {
        return;
      }

      setReport(generatedReport);
      setLoading(false);
    } catch (nextError) {
      if (!isRequestCurrent(requestId, appid)) {
        return;
      }

      setError(getErrorMessage(nextError));
      setLoading(false);
    }
  }

  async function refresh() {
    const appid = game.appid;
    const requestId = beginRequest();

    try {
      const generatedReport = await generateGameAnalysis(appid, true);
      if (!isRequestCurrent(requestId, appid)) {
        return;
      }

      setReport(generatedReport);
      setLoading(false);
    } catch (nextError) {
      if (!isRequestCurrent(requestId, appid)) {
        return;
      }

      setError(getErrorMessage(nextError));
      setLoading(false);
    }
  }

  function toggleExpanded() {
    setExpanded((value) => !value);
  }

  function beginRequest() {
    const requestId = activeRequestIdRef.current + 1;
    activeRequestIdRef.current = requestId;
    setLoading(true);
    setError(null);
    return requestId;
  }

  function isRequestCurrent(requestId: number, appid: number) {
    return (
      mountedRef.current &&
      activeRequestIdRef.current === requestId &&
      currentAppidRef.current === appid
    );
  }

  return {
    report,
    loading,
    error,
    expanded,
    refresh,
    toggleExpanded,
  };
}

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : "AI 评估暂时不可用，请稍后再试。";
}
