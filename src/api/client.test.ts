import { beforeEach, describe, expect, it } from "vitest";
import {
  __resetMockGameAnalysisCacheForTests,
  generateGameAnalysis,
  getGameAnalysis,
  isTauriRuntime,
} from "./client";

describe("game analysis client", () => {
  beforeEach(() => {
    __resetMockGameAnalysisCacheForTests();
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
});
