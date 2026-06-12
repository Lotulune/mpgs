import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  __resetMockGameAnalysisCacheForTests,
  assessGameWithAi,
  discoverSteamGames,
  generateGameAnalysis,
  getDashboard,
  getGameDetail,
  getGameAnalysis,
  getUserCollections,
  isTauriRuntime,
  recommendGamesWithAi,
  refreshAllGameAnalyses,
  retryAiAnalysisJob,
  setGameUserState,
  startClassicDiscoveryTask,
  startDiscoveryTask,
  syncSeedGames,
} from "./client";
import { mockDashboard } from "../data/mockDashboard";
import {
  clearCurrentServiceConnection,
  saveCurrentServiceConnection,
} from "../domain/serviceConnectionStorage";
import type { GameAnalysisReport, ServiceInfo } from "../types";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

const compatibleInfo: ServiceInfo = {
  serviceInstanceId: "018fb770-8998-7699-a6e4-b7b59f2f9c01",
  serviceName: "MPGS Test Service",
  serviceVersion: "0.1.0",
  apiVersion: "v1",
  publicCatalogStatus: "ready",
  capabilities: ["public_catalog_read"],
};

function configureServiceConnection() {
  saveCurrentServiceConnection({
    baseUrl: "https://mpgs.example.test",
    info: compatibleInfo,
    validatedAt: "2026-06-08T00:00:00.000Z",
  });
}

function publicGameItem(
  overrides: Partial<{
    appid: number;
    capsuleUrl: string;
    currentPlayers: number | null;
    demoStatus: string;
    discountPercent: number | null;
    isAdultContent: boolean;
    isFree: boolean;
    multiplayerModes: string[];
    name: string;
    positiveReviewPct: number | null;
    priceText: string | null;
    recommendationScore: number | null;
    releaseDate: string | null;
    releaseDateText: string;
    releaseState: string;
    reviewSnippets: Array<{
      playtimeHours?: number | null;
      review: string;
      votedUp: boolean;
    }>;
    section: string;
    shortDescription: string | null;
    storeScreenshotUrls: string[];
    supportedLanguages: string[];
    tags: string[];
    totalReviews: number | null;
    updatedAt: string;
  }>,
) {
  return {
    appid: 3744430,
    capsuleUrl: "https://assets.example.test/together-moon-escape/header.jpg",
    currentPlayers: 340,
    demoStatus: "released_with_demo",
    discountPercent: 15,
    isAdultContent: false,
    isFree: false,
    multiplayerModes: ["Online Co-op", "Shared/Split Screen Co-op"],
    name: "Together Moon Escape",
    positiveReviewPct: 93,
    priceText: "$12.99",
    recommendationScore: 92,
    releaseDate: "2026-06-01",
    releaseDateText: "Jun 1, 2026",
    releaseState: "released",
    reviewSnippets: [
      {
        playtimeHours: 8.5,
        review: "A tidy co-op escape room that makes every switch callout matter.",
        votedUp: true,
      },
    ],
    section: "new",
    shortDescription: "A cooperative moonbase escape room built for two players.",
    storeScreenshotUrls: [
      "https://assets.example.test/together-moon-escape/screen-1.jpg",
    ],
    supportedLanguages: ["English", "Simplified Chinese"],
    tags: ["Co-op", "Puzzle", "Escape Room"],
    totalReviews: 1280,
    updatedAt: "2026-06-08T00:00:00Z",
    ...overrides,
  };
}

const togetherMoonEscape = publicGameItem({});
const deepRockGalactic = publicGameItem({
  appid: 548430,
  capsuleUrl: "https://assets.example.test/deep-rock/header.jpg",
  currentPlayers: 8600,
  demoStatus: "released",
  discountPercent: null,
  multiplayerModes: ["Online Co-op", "Co-op"],
  name: "Deep Rock Galactic",
  positiveReviewPct: 97,
  priceText: "$29.99",
  recommendationScore: 95,
  releaseDate: "2020-05-13",
  releaseDateText: "May 13, 2020",
  reviewSnippets: [
    {
      playtimeHours: 120,
      review: "Still the cleanest four-player mining loop around.",
      votedUp: true,
    },
  ],
  section: "classic",
  shortDescription: "A four-player mining shooter with strong co-op roles.",
  storeScreenshotUrls: ["https://assets.example.test/deep-rock/screen-1.jpg"],
  tags: ["Co-op", "FPS", "Mining"],
  totalReviews: 250000,
});

const publicHomePayload = {
  status: "ready",
  totalGames: 2,
  sections: {
    newlyPublished: [togetherMoonEscape],
    highConfidence: [deepRockGalactic],
    recentlyAdded: [],
  },
};

const publicGamesPayload = {
  items: [togetherMoonEscape, deepRockGalactic],
  page: { limit: 100, offset: 0, total: 2 },
};

function fetchJson(
  value: unknown,
  status = 200,
  headers: Record<string, string> = {},
) {
  const normalizedHeaders = new Map(
    Object.entries(headers).map(([key, headerValue]) => [
      key.toLowerCase(),
      headerValue,
    ]),
  );

  return Promise.resolve({
    ok: status >= 200 && status < 300,
    status,
    headers: {
      get: (name: string) => normalizedHeaders.get(name.toLowerCase()) ?? null,
    },
    json: async () => value,
  } as Response);
}

function installPublicServiceFetch(report?: GameAnalysisReport) {
  const fetchMock = vi.fn((input: RequestInfo | URL) => {
    const url = String(input);
    if (url === "https://mpgs.example.test/api/v1/discovery-home") {
      return fetchJson(publicHomePayload);
    }

    if (url === "https://mpgs.example.test/api/v1/games?limit=100&offset=0") {
      return fetchJson(publicGamesPayload);
    }

    if (url === "https://mpgs.example.test/api/v1/games/548430/analysis" && report) {
      return fetchJson({
        appid: 548430,
        generatedAt: report.generatedAt,
        report,
      });
    }

    if (url === "https://mpgs.example.test/api/v1/games/548430/analysis") {
      return fetchJson({ error: { code: "public_game_not_found" } }, 404);
    }

    throw new Error(`Unexpected URL: ${url}`);
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function setTauriRuntime(enabled: boolean) {
  if (enabled) {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    return;
  }

  delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

describe("game analysis client", () => {
  beforeEach(() => {
    __resetMockGameAnalysisCacheForTests();
    window.localStorage.clear();
    clearCurrentServiceConnection();
    invokeMock.mockReset();
    setTauriRuntime(false);
    vi.unstubAllGlobals();
  });

  it("returns null before a browser-mode report has been generated", async () => {
    expect(isTauriRuntime()).toBe(false);

    await expect(getGameAnalysis(3744430)).resolves.toBeNull();
  });

  it("caches a browser-mode report after generation", async () => {
    const generated = await generateGameAnalysis(3087930, false);
    const cached = await getGameAnalysis(3087930);

    expect(generated.appid).toBe(3087930);
    expect(generated.overview.length).toBeGreaterThan(0);
    expect(cached).toEqual(generated);
  });

  it("overwrites the cached browser-mode report when force refresh is enabled", async () => {
    const first = await generateGameAnalysis(548430, false);
    const second = await generateGameAnalysis(548430, true);
    const cached = await getGameAnalysis(548430);

    expect(second.appid).toBe(548430);
    expect(second.generatedAt).not.toBe(first.generatedAt);
    expect(second.overview).not.toBe(first.overview);
    expect(cached).toEqual(second);
  });

  it("fetches public dashboard data from the configured service instead of Tauri commands", async () => {
    configureServiceConnection();
    const fetchMock = installPublicServiceFetch();

    const dashboard = await getDashboard();

    expect(invokeMock).not.toHaveBeenCalled();
    expect(fetchMock).toHaveBeenCalledWith(
      "https://mpgs.example.test/api/v1/discovery-home",
      expect.objectContaining({ method: "GET" }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "https://mpgs.example.test/api/v1/games?limit=100&offset=0",
      expect.objectContaining({ method: "GET" }),
    );
    expect(dashboard.newGames[0]).toMatchObject({
      appid: 3744430,
      name: "Together Moon Escape",
      shortDescription: "A cooperative moonbase escape room built for two players.",
      section: "new",
      releaseDate: "2026-06-01",
      releaseDateText: "Jun 1, 2026",
      releaseState: "released",
      demoStatus: "released_with_demo",
      supportedLanguages: ["English", "Simplified Chinese"],
      isAdultContent: false,
      isFree: false,
      priceText: "$12.99",
      discountPercent: 15,
      positiveReviewPct: 93,
      totalReviews: 1280,
      currentPlayers: 340,
      recommendationScore: 92,
      aiScore: 92,
      aiSummary: "A cooperative moonbase escape room built for two players.",
      capsuleUrl: "https://assets.example.test/together-moon-escape/header.jpg",
      storeScreenshotUrls: [
        "https://assets.example.test/together-moon-escape/screen-1.jpg",
      ],
      tags: ["Co-op", "Puzzle", "Escape Room"],
      multiplayerModes: ["Online Co-op", "Shared/Split Screen Co-op"],
      reviewSnippets: [
        {
          playtimeHours: 8.5,
          review: "A tidy co-op escape room that makes every switch callout matter.",
          votedUp: true,
        },
      ],
      userState: {
        favorite: false,
        wishlist: false,
        followed: false,
        viewed: false,
      },
    });
    expect(dashboard.classics[0]).toMatchObject({
      appid: 548430,
      name: "Deep Rock Galactic",
      capsuleUrl: "https://assets.example.test/deep-rock/header.jpg",
      tags: ["Co-op", "FPS", "Mining"],
      multiplayerModes: ["Online Co-op", "Co-op"],
      positiveReviewPct: 97,
      totalReviews: 250000,
      currentPlayers: 8600,
      reviewSnippets: [
        {
          playtimeHours: 120,
          review: "Still the cleanest four-player mining loop around.",
          votedUp: true,
        },
      ],
    });
    expect(dashboard.stats).toMatchObject({
      totalGames: 2,
      newGamesCount: 1,
      classicGamesCount: 1,
      dataSource: "公共发现服务：MPGS Test Service",
    });
    expect(dashboard.config.onboardingCompleted).toBe(true);
  });

  it("uses ETags and cached public dashboard bodies for 304 responses", async () => {
    configureServiceConnection();
    let homeRequests = 0;
    let gamesRequests = 0;
    const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "https://mpgs.example.test/api/v1/discovery-home") {
        homeRequests += 1;
        if (homeRequests === 1) {
          return fetchJson(publicHomePayload, 200, { ETag: '"home-rev-1"' });
        }

        expect(init?.headers).toMatchObject({
          "If-None-Match": '"home-rev-1"',
        });
        return fetchJson(null, 304);
      }

      if (url === "https://mpgs.example.test/api/v1/games?limit=100&offset=0") {
        gamesRequests += 1;
        if (gamesRequests === 1) {
          return fetchJson(publicGamesPayload, 200, { ETag: '"games-rev-1"' });
        }

        expect(init?.headers).toMatchObject({
          "If-None-Match": '"games-rev-1"',
        });
        return fetchJson(null, 304);
      }

      throw new Error(`Unexpected URL: ${url}`);
    });
    vi.stubGlobal("fetch", fetchMock);

    const firstDashboard = await getDashboard();
    const secondDashboard = await getDashboard();

    expect(firstDashboard.newGames[0].name).toBe("Together Moon Escape");
    expect(secondDashboard.newGames[0].name).toBe("Together Moon Escape");
    expect(secondDashboard.classics[0].name).toBe("Deep Rock Galactic");
    expect(fetchMock).toHaveBeenCalledTimes(4);
  });

  it("falls back to the cached public dashboard snapshot when the service is unreachable", async () => {
    configureServiceConnection();
    const fetchMock = vi
      .fn()
      .mockImplementationOnce(() =>
        fetchJson(publicHomePayload, 200, { ETag: '"home-rev-1"' }),
      )
      .mockImplementationOnce(() =>
        fetchJson(publicGamesPayload, 200, { ETag: '"games-rev-1"' }),
      )
      .mockRejectedValue(new Error("offline"));
    vi.stubGlobal("fetch", fetchMock);

    await expect(getDashboard()).resolves.toMatchObject({
      stats: { totalGames: 2 },
    });

    await expect(getDashboard()).resolves.toMatchObject({
      stats: {
        totalGames: 2,
        dataSource: "公共发现服务：MPGS Test Service",
      },
      classics: [{ appid: 548430, name: "Deep Rock Galactic" }],
    });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("keeps the old Tauri dashboard command as a fallback when no service is configured", async () => {
    setTauriRuntime(true);
    invokeMock.mockResolvedValueOnce(mockDashboard);
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    await expect(getDashboard()).resolves.toBe(mockDashboard);

    expect(invokeMock).toHaveBeenCalledWith("get_dashboard");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("reads public analysis from the configured service with GET requests only", async () => {
    configureServiceConnection();
    const report: GameAnalysisReport = {
      appid: 548430,
      generatedAt: "2026-06-08T00:00:00Z",
      source: "rule",
      confidence: "high",
      overallScore: 95,
      overview: "Public rule analysis.",
      dimensionScores: [],
      strengths: [],
      risks: [],
      evidence: [],
      reviewEvidence: [],
    };
    const fetchMock = installPublicServiceFetch(report);

    await expect(getGameAnalysis(548430)).resolves.toEqual(report);
    await expect(generateGameAnalysis(548430, true)).resolves.toEqual(report);

    expect(invokeMock).not.toHaveBeenCalled();
    expect(fetchMock).toHaveBeenCalledWith(
      "https://mpgs.example.test/api/v1/games/548430/analysis",
      expect.objectContaining({ method: "GET" }),
    );
    expect(fetchMock).not.toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("reads public game detail from the configured service without Tauri commands", async () => {
    configureServiceConnection();
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "https://mpgs.example.test/api/v1/discovery-home") {
        return fetchJson(publicHomePayload);
      }

      if (url === "https://mpgs.example.test/api/v1/games?limit=100&offset=0") {
        return fetchJson(publicGamesPayload);
      }

      if (url === "https://mpgs.example.test/api/v1/games/548430") {
        return fetchJson({
          game: publicGameItem({
            appid: 548430,
            capsuleUrl: "https://assets.example.test/deep-rock/detail-header.jpg",
            currentPlayers: 9100,
            multiplayerModes: ["Online Co-op", "Co-op", "Cross-Platform Multiplayer"],
            name: "Deep Rock Galactic - Public Detail",
            positiveReviewPct: 98,
            recommendationScore: 97,
            reviewSnippets: [
              {
                playtimeHours: 140,
                review: "The detail payload keeps the squad loop fresh.",
                votedUp: true,
              },
            ],
            section: "classic",
            shortDescription: "Detail payload description from the public service.",
            storeScreenshotUrls: [
              "https://assets.example.test/deep-rock/detail-screen-1.jpg",
            ],
            tags: ["Co-op", "FPS", "Extraction"],
            totalReviews: 260000,
            updatedAt: "2026-06-08T01:00:00Z",
          }),
        });
      }

      throw new Error(`Unexpected URL: ${url}`);
    });
    vi.stubGlobal("fetch", fetchMock);

    const baseGame = (await getDashboard()).classics[0];

    await expect(getGameDetail(baseGame)).resolves.toMatchObject({
      appid: 548430,
      name: "Deep Rock Galactic - Public Detail",
      shortDescription: "Detail payload description from the public service.",
      recommendationScore: 97,
      aiScore: 97,
      aiSummary: "Detail payload description from the public service.",
      capsuleUrl: "https://assets.example.test/deep-rock/detail-header.jpg",
      storeScreenshotUrls: [
        "https://assets.example.test/deep-rock/detail-screen-1.jpg",
      ],
      tags: ["Co-op", "FPS", "Extraction"],
      multiplayerModes: ["Online Co-op", "Co-op", "Cross-Platform Multiplayer"],
      positiveReviewPct: 98,
      totalReviews: 260000,
      currentPlayers: 9100,
      reviewSnippets: [
        {
          playtimeHours: 140,
          review: "The detail payload keeps the squad loop fresh.",
          votedUp: true,
        },
      ],
      userState: {
        favorite: false,
        wishlist: false,
        followed: false,
        viewed: false,
      },
    });

    expect(invokeMock).not.toHaveBeenCalled();
    expect(fetchMock).toHaveBeenCalledWith(
      "https://mpgs.example.test/api/v1/games/548430",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("keeps personal game state local and merges it into public REST dashboards", async () => {
    configureServiceConnection();
    const fetchMock = installPublicServiceFetch();

    await expect(setGameUserState(548430, { favorite: true })).resolves.toMatchObject({
      favorite: true,
    });
    const dashboard = await getDashboard();

    expect(fetchMock).not.toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ method: "POST" }),
    );
    expect(dashboard.classics[0].userState).toMatchObject({
      favorite: true,
      wishlist: false,
      followed: false,
      viewed: false,
    });
    expect(dashboard.collections.favorites).toHaveLength(1);
    expect(dashboard.collections.favorites[0]).toMatchObject({
      appid: 548430,
      name: "Deep Rock Galactic",
    });
  });

  it("reads service-mode collections from public REST data instead of local Tauri SQLite", async () => {
    configureServiceConnection();
    setTauriRuntime(true);
    const fetchMock = installPublicServiceFetch();

    await setGameUserState(548430, { wishlist: true });
    const collections = await getUserCollections();

    expect(invokeMock).not.toHaveBeenCalled();
    expect(fetchMock).toHaveBeenCalledWith(
      "https://mpgs.example.test/api/v1/discovery-home",
      expect.objectContaining({ method: "GET" }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "https://mpgs.example.test/api/v1/games?limit=100&offset=0",
      expect.objectContaining({ method: "GET" }),
    );
    expect(collections.wishlist).toHaveLength(1);
    expect(collections.wishlist[0]).toMatchObject({
      appid: 548430,
      name: "Deep Rock Galactic",
    });
  });

  it("freezes legacy client-side discovery, sync, and AI command paths when a public service is configured", async () => {
    configureServiceConnection();
    setTauriRuntime(true);

    const blockedActions = [
      () => syncSeedGames("full"),
      () => discoverSteamGames(1, 10),
      () => assessGameWithAi(548430),
      () =>
        recommendGamesWithAi({
          prompt: "recommend co-op games",
          contextMessages: [],
          limit: 3,
        }),
      () => refreshAllGameAnalyses(2),
      () => retryAiAnalysisJob(548430),
      () =>
        startDiscoveryTask({
          syncMode: "full",
          targetAddedGames: 5,
          pageSize: 25,
        }),
      () => startClassicDiscoveryTask(2),
    ];

    for (const runAction of blockedActions) {
      await expect(runAction()).rejects.toThrow(
        "公共发现服务模式下，客户端不会执行本地同步、发现或 AI 任务。",
      );
    }

    expect(invokeMock).not.toHaveBeenCalled();
  });
});
