import { describe, expect, it } from "vitest";
import {
  buildEasyRoutePlan,
  preferChatModelIds,
  singleModelRoutes,
  taskLabel,
} from "../src/app/customAiRoutes";

describe("buildEasyRoutePlan", () => {
  it("assigns every task to the user-selected model without name heuristics", () => {
    const plan = buildEasyRoutePlan("vendor-xyz-chat-7b");
    expect(plan.preset).toBe("easy");
    expect(plan.model).toBe("vendor-xyz-chat-7b");
    expect(plan.fallback_model).toBeNull();
    expect(plan.routes.length).toBeGreaterThan(0);
    for (const route of plan.routes) {
      expect(route.primary_model).toBe("vendor-xyz-chat-7b");
      expect(route.fallback_models).toEqual([]);
    }
    expect(plan.notes.some((n) => n.includes("vendor-xyz-chat-7b"))).toBe(true);
  });

  it("does not pick different models based on gpt/claude-style names", () => {
    // Even if the caller only passes one selection, plan stays uniform.
    // Name patterns must never split tasks.
    const plan = buildEasyRoutePlan("gpt-4o");
    const models = new Set(plan.routes.map((r) => r.primary_model));
    expect(models.size).toBe(1);
    expect(models.has("gpt-4o")).toBe(true);
  });

  it("returns empty routes when model is blank", () => {
    const plan = buildEasyRoutePlan("  ");
    expect(plan.model).toBe("");
    expect(plan.routes).toEqual([]);
  });
});

describe("singleModelRoutes", () => {
  it("covers core NL tasks", () => {
    const routes = singleModelRoutes("m1");
    expect(routes.map((r) => r.task)).toEqual(
      expect.arrayContaining(["intent_parse", "rank_explain", "compare_games"]),
    );
  });
});

describe("preferChatModelIds", () => {
  it("filters obvious non-chat assets but keeps all when only those exist", () => {
    expect(preferChatModelIds(["chat-a", "dall-e-3", "embed-small"])).toEqual(["chat-a"]);
    expect(preferChatModelIds(["whisper-1", "tts-1"])).toEqual(["whisper-1", "tts-1"]);
  });
});

describe("taskLabel", () => {
  it("maps known tasks and falls back to id", () => {
    expect(taskLabel("intent_parse")).toContain("理解");
    expect(taskLabel("unknown_task_x")).toBe("unknown_task_x");
  });
});
