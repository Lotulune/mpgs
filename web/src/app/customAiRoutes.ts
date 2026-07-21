// Client-side helpers for custom multi-model routing (device-local keys only).
// Do NOT assign models by name heuristics — vendor model ids are not comparable.

export type CustomRoutingPreset = "easy" | "advanced" | "single";

export interface CustomTaskRoute {
  task: string;
  primary_model: string;
  fallback_models: string[];
}

export interface EasyRoutePlan {
  preset: CustomRoutingPreset;
  /** Default / construction-time model. */
  model: string;
  fallback_model: string | null;
  routes: CustomTaskRoute[];
  notes: string[];
}

/** Tasks that custom routing may assign. */
export const CUSTOM_ROUTE_TASKS = [
  "intent_parse",
  "rank_explain",
  "compare_games",
  "group_advice",
  "game_summary",
] as const;

export function singleModelRoutes(model: string): CustomTaskRoute[] {
  const primary = model.trim();
  return CUSTOM_ROUTE_TASKS.map((task) => ({
    task,
    primary_model: primary,
    fallback_models: [],
  }));
}

/**
 * 一键省心: every task uses the user-selected model.
 * No name-based capability guessing across vendors.
 */
export function buildEasyRoutePlan(selectedModel: string): EasyRoutePlan {
  const model = selectedModel.trim();
  if (!model) {
    return {
      preset: "single",
      model: "",
      fallback_model: null,
      routes: [],
      notes: ["请先选择或填写一个模型。"],
    };
  }
  return {
    preset: "easy",
    model,
    fallback_model: null,
    routes: singleModelRoutes(model),
    notes: [
      `所有任务统一使用「${model}」（不按名称猜测能力；需要分任务时请改用高级配置）。`,
    ],
  };
}

/** Human labels for task ids shown in settings. */
export const TASK_LABELS: Record<string, string> = {
  intent_parse: "理解你的话",
  rank_explain: "推荐理由",
  compare_games: "游戏比较",
  group_advice: "小组建议",
  game_summary: "游戏总结",
  data_quality: "数据质检",
};

export function taskLabel(task: string): string {
  return TASK_LABELS[task] ?? task;
}

/** Prefer chat-like ids for the picker default; never invent assignments. */
export function preferChatModelIds(modelIds: string[]): string[] {
  const imageLike = /imagine|image|dall|vision|tts|whisper|embed|moderation|rerank/i;
  const unique = [...new Set(modelIds.map((m) => m.trim()).filter(Boolean))];
  const chat = unique.filter((id) => !imageLike.test(id));
  return chat.length > 0 ? chat : unique;
}
