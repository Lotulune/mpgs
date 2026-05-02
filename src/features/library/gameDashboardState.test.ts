import { describe, expect, it } from "vitest";
import { mockDashboard } from "../../data/mockDashboard";
import {
  applyGameAnalysisSnapshotToDashboard,
  applyGameAnalysisSnapshotToGame,
  snapshotFromReport,
} from "./gameDashboardState";

describe("gameDashboardState", () => {
  it("applies the latest analysis snapshot to every matching dashboard slice", () => {
    const target = structuredClone(mockDashboard.newGames[0]);
    const dashboard = structuredClone(mockDashboard);
    dashboard.collections.favorites = [structuredClone(target)];
    dashboard.recentDiscoveries = [structuredClone(target)];
    const snapshot = {
      appid: target.appid,
      aiScore: 97,
      aiSummary: "新的 AI 评测摘要",
    };

    const updated = applyGameAnalysisSnapshotToDashboard(dashboard, snapshot);

    expect(updated.newGames[0].aiScore).toBe(97);
    expect(updated.newGames[0].aiSummary).toBe("新的 AI 评测摘要");
    expect(updated.collections.favorites[0].aiScore).toBe(97);
    expect(updated.recentDiscoveries[0].aiScore).toBe(97);
  });

  it("keeps unrelated games untouched", () => {
    const source = structuredClone(mockDashboard.classics[0]);
    const snapshot = {
      appid: source.appid + 1,
      aiScore: 99,
      aiSummary: "should not apply",
    };

    const updated = applyGameAnalysisSnapshotToGame(source, snapshot);

    expect(updated).toEqual(source);
  });

  it("builds an analysis snapshot directly from a report", () => {
    expect(
      snapshotFromReport({
        appid: 123,
        overallScore: 61,
        recommendationScore: 91,
        overview: "立即同步到卡片",
      } as unknown as Parameters<typeof snapshotFromReport>[0]),
    ).toEqual({
      appid: 123,
      aiScore: 91,
      aiSummary: "立即同步到卡片",
    });
  });
});
