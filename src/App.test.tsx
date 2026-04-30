// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { mockDashboard } from "./data/mockDashboard";
import type { GameAnalysisReport } from "./types";

const assessGameWithAiMock = vi.fn();
const getDashboardMock = vi.fn();
const getGameAnalysisMock = vi.fn();
const generateGameAnalysisMock = vi.fn();
const syncSeedGamesMock = vi.fn();

vi.mock("./api/client", async () => {
  const actual = await vi.importActual<typeof import("./api/client")>("./api/client");

  return {
    ...actual,
    assessGameWithAi: (...args: unknown[]) => assessGameWithAiMock(...args),
    getDashboard: () => getDashboardMock(),
    getGameAnalysis: (...args: unknown[]) => getGameAnalysisMock(...args),
    generateGameAnalysis: (...args: unknown[]) => generateGameAnalysisMock(...args),
    previewSteamAppList: vi.fn(),
    saveConfig: vi.fn(),
    setGameUserState: vi.fn(),
    syncSeedGames: (...args: unknown[]) => syncSeedGamesMock(...args),
  };
});

function buildDashboard() {
  return structuredClone(mockDashboard);
}

function buildLowActivityDiscoveryDashboard() {
  const dashboard = structuredClone(mockDashboard);
  const lowActivityGame = {
    ...dashboard.newGames[0],
    appid: 4999001,
    name: "Quiet Co-op Debut",
    positiveReviewPct: 0,
    totalReviews: 0,
    currentPlayers: 0,
    recommendationScore: 12,
    aiScore: 12,
    userState: {
      favorite: false,
      wishlist: false,
      followed: false,
      viewed: false,
      updatedAt: null,
    },
  };

  dashboard.newGames = [lowActivityGame];
  dashboard.classics = [];
  dashboard.upcoming = [];
  dashboard.recentDiscoveries = [lowActivityGame];
  dashboard.collections = {
    favorites: [],
    wishlist: [],
    followed: [],
    history: [],
  };
  dashboard.stats = {
    ...dashboard.stats,
    seedCount: 1,
    totalGames: 1,
    newGamesCount: 1,
    classicGamesCount: 0,
  };

  return dashboard;
}

function buildBackfillDashboard() {
  const dashboard = structuredClone(mockDashboard);
  dashboard.stats = {
    ...dashboard.stats,
    backfillPendingCount: 3,
    backfillRunning: true,
    backfillCurrentAppid: 730123,
    backfillCurrentAttempt: 1,
  };

  return dashboard;
}

function buildAnalysisReport(
  appid: number,
  overrides: Partial<GameAnalysisReport> = {},
): GameAnalysisReport {
  return {
    appid,
    generatedAt: "2026-04-30T12:45:00.000Z",
    source: "hybrid",
    confidence: "high",
    overallScore: 92,
    overview: "打开详情页后应直接显示缓存分析。",
    dimensionScores: [
      {
        key: "approachability",
        label: "易上手度",
        score: 88,
        reason: "回归测试夹具。",
      },
      {
        key: "multiplayer_fun",
        label: "联机乐趣",
        score: 94,
        reason: "回归测试夹具。",
      },
      {
        key: "content_depth",
        label: "内容深度",
        score: 86,
        reason: "回归测试夹具。",
      },
      {
        key: "reputation_stability",
        label: "口碑稳定性",
        score: 95,
        reason: "回归测试夹具。",
      },
      {
        key: "activity_health",
        label: "活跃健康度",
        score: 90,
        reason: "回归测试夹具。",
      },
    ],
    strengths: [{ title: "缓存可见", reason: "详情页直接显示摘要。" }],
    risks: [{ title: "无", reason: "纯回归夹具。" }],
    evidence: [
      {
        kind: "positive_review_pct",
        label: "好评率",
        value: "97%",
        interpretation: "回归测试夹具。",
      },
    ],
    reviewEvidence: [],
    ...overrides,
  };
}

function getGameTitles(sectionHeading: string) {
  const heading = screen.getByRole("heading", { name: sectionHeading });
  const section = heading.closest(".game-section");

  if (!(section instanceof HTMLElement)) {
    throw new Error(`Missing game section for ${sectionHeading}`);
  }

  return within(section)
    .getAllByRole("heading", { level: 3 })
    .map((node) => node.textContent);
}

describe("App dashboard interactions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getDashboardMock.mockResolvedValue(buildDashboard());
    assessGameWithAiMock.mockResolvedValue({
      appid: 0,
      score: 80,
      summary: "AI 评估结果",
      bestFor: [],
      risks: [],
    });
    getGameAnalysisMock.mockResolvedValue(null);
    generateGameAnalysisMock.mockImplementation(async (appid: number) =>
      buildAnalysisReport(appid),
    );
    syncSeedGamesMock.mockResolvedValue({
      updatedGames: 0,
      failedGames: 0,
      message: "已启动 Steam 同步任务。",
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    cleanup();
  });

  it("renders sort controls as direct-action buttons instead of a native combobox", async () => {
    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });

    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "综合排序" })).toHaveAttribute("aria-pressed", "true");
  });

  it("reorders the new games section when clicking the players sort button", async () => {
    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });

    expect(getGameTitles("新游区")).toEqual([
      "Together Moon Escape",
      "Pebble Knights",
      "Burglin' Gnomes",
      "Void Crew",
    ]);

    fireEvent.click(screen.getByRole("button", { name: "游玩人数" }));

    await waitFor(() =>
      expect(getGameTitles("新游区")).toEqual([
        "Void Crew",
        "Together Moon Escape",
        "Pebble Knights",
        "Burglin' Gnomes",
      ]),
    );

    expect(screen.getByRole("button", { name: "游玩人数" })).toHaveAttribute("aria-pressed", "true");
  });

  it("opens the full new-games view when clicking the first 查看全部 action", async () => {
    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });

    fireEvent.click(screen.getAllByRole("button", { name: "查看全部 〉" })[0]);

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "新游区" })).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: "精品老游区" })).not.toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: "最近发现" })).not.toBeInTheDocument();
    });
  });

  it("filters dashboard cards when clicking the demo status tabs", async () => {
    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });

    fireEvent.click(screen.getByRole("button", { name: "仅 Demo" }));

    await waitFor(() => {
      expect(getGameTitles("新游区")).toEqual([
        "Together Moon Escape",
        "Pebble Knights",
      ]);
      expect(screen.queryByRole("heading", { name: "精品老游区" })).not.toBeInTheDocument();
    });
  });

  it("applies tag selections from the filter page", async () => {
    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });

    fireEvent.click(screen.getByRole("button", { name: "筛选器" }));
    await screen.findByRole("button", { name: "应用筛选" });

    fireEvent.click(screen.getByRole("button", { name: "射击" }));
    fireEvent.click(screen.getByRole("button", { name: "应用筛选" }));

    await waitFor(() => {
      expect(getGameTitles("精品老游区")).toEqual([
        "Deep Rock Galactic",
        "Left 4 Dead 2",
      ]);
      expect(screen.queryByRole("heading", { name: "新游区" })).not.toBeInTheDocument();
    });
  });

  it("filters immediately from the right-rail quick tags", async () => {
    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });

    fireEvent.click(screen.getByRole("button", { name: "解谜" }));

    await waitFor(() =>
      expect(getGameTitles("新游区")).toEqual(["Together Moon Escape"]),
    );
  });

  it("keeps newly discovered low-activity games visible by default", async () => {
    getDashboardMock.mockResolvedValue(buildLowActivityDiscoveryDashboard());

    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });
    expect(screen.getByRole("heading", { name: "最近发现" })).toBeInTheDocument();
    expect(screen.getAllByText("Quiet Co-op Debut").length).toBeGreaterThan(0);
  });

  it("polls for dashboard refresh while metadata backfill is running", async () => {
    vi.useFakeTimers();
    getDashboardMock.mockResolvedValue(buildBackfillDashboard());

    render(<App />);

    await act(async () => {
      await Promise.resolve();
    });

    expect(screen.getByRole("heading", { name: "新游区" })).toBeInTheDocument();
    expect(getDashboardMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      vi.advanceTimersByTime(2_200);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(getDashboardMock).toHaveBeenCalledTimes(2);
  });

  it("routes full and quick sync requests with their selected mode", async () => {
    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });

    fireEvent.click(screen.getByRole("button", { name: "完整同步" }));
    expect(syncSeedGamesMock).toHaveBeenNthCalledWith(1, "full");

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "快速同步" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "快速同步" }));
    expect(syncSeedGamesMock).toHaveBeenNthCalledWith(2, "quick");
  });

  it("opens a dashboard game card into detail view and shows cached analysis", async () => {
    const dashboard = buildDashboard();
    const report = buildAnalysisReport(dashboard.newGames[0].appid);
    getDashboardMock.mockResolvedValue(dashboard);
    getGameAnalysisMock.mockResolvedValue(report);
    generateGameAnalysisMock.mockResolvedValue(report);

    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });

    const newGamesSection = screen
      .getByRole("heading", { name: "新游区" })
      .closest(".game-section");
    if (!(newGamesSection instanceof HTMLElement)) {
      throw new Error("Missing 新游区 section");
    }

    fireEvent.click(
      within(newGamesSection).getByRole("button", {
        name: new RegExp(dashboard.newGames[0].name, "i"),
      }),
    );

    expect(await screen.findByText(report.overview)).toBeInTheDocument();
    expect(generateGameAnalysisMock).not.toHaveBeenCalled();
  });

  it("refreshes the selected detail game from the latest dashboard payload", async () => {
    const initialDashboard = buildDashboard();
    const updatedDashboard = buildDashboard();
    const targetAppid = initialDashboard.newGames[0].appid;
    const refreshedDescription = "刷新后的详情简介，应替换掉旧的选中游戏对象。";

    initialDashboard.newGames[0] = {
      ...initialDashboard.newGames[0],
      shortDescription: "旧的详情简介。",
    };
    updatedDashboard.newGames[0] = {
      ...updatedDashboard.newGames[0],
      shortDescription: refreshedDescription,
    };

    getDashboardMock
      .mockResolvedValueOnce(initialDashboard)
      .mockResolvedValue(updatedDashboard);
    getGameAnalysisMock.mockResolvedValue(buildAnalysisReport(targetAppid));

    render(<App />);

    await screen.findByRole("heading", { name: "新游区" });

    const newGamesSection = screen
      .getByRole("heading", { name: "新游区" })
      .closest(".game-section");
    if (!(newGamesSection instanceof HTMLElement)) {
      throw new Error("Missing 新游区 section");
    }

    fireEvent.click(
      within(newGamesSection).getByRole("button", {
        name: new RegExp(initialDashboard.newGames[0].name, "i"),
      }),
    );

    expect(await screen.findByText("旧的详情简介。")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByText(refreshedDescription)).toBeInTheDocument(),
    );
  });
});
