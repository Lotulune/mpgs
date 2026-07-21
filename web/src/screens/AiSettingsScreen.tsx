import { useEffect, useMemo, useState } from "react";
import { ApiError } from "../api/client";
import type { AiSettings } from "../api/types";
import { apiClient } from "../app/runtime";
import {
  buildEasyRoutePlan,
  preferChatModelIds,
  singleModelRoutes,
  taskLabel,
  type CustomRoutingPreset,
  type CustomTaskRoute,
} from "../app/customAiRoutes";
import {
  loadLocalCustomAiSettings,
  removeLocalCustomAiSettings,
  saveLocalCustomAiSettings,
  type LocalCustomAiSettings,
} from "../app/localAiSettings";
import { useToast } from "../app/ToastProvider";

type Mode = AiSettings["mode"];

function aiErrorMessage(
  error: unknown,
  action: "load" | "test" | "save" | "delete" | "discover",
): string {
  if (error instanceof ApiError) {
    if (error.code === "temporarily_unavailable") {
      return "服务端尚未启用自定义 AI 凭据加密，请联系管理员配置后重试。";
    }
    if (error.code === "ai_connection_failed") {
      return "连接测试失败，请检查 HTTPS 地址、模型名称和 API Key。";
    }
    if (error.code === "unauthenticated") return "登录状态已失效，请重新登录。";
    if (error.code === "invalid_argument") return `配置不正确：${error.message}`;
    if (error.offline) return "当前无法连接服务器，请检查网络。";
    return `${error.message}（${error.code}）`;
  }
  if (action === "load") return "无法加载 AI 设置。";
  if (action === "test") return "无法连接到该服务。";
  if (action === "discover") return "无法拉取上游模型列表。";
  if (action === "delete") return "无法删除自定义密钥。";
  return "AI 设置保存失败。";
}

function RoutePreview({
  routes,
  title,
  note,
}: {
  routes: CustomTaskRoute[];
  title: string;
  note: string;
}) {
  return (
    <div className="ai-routes" aria-label={title}>
      <p className="cal-note">{note}</p>
      <ul className="ai-route-list">
        {routes.map((route) => (
          <li key={route.task} className="ai-route-row">
            <strong title={route.task}>{taskLabel(route.task)}</strong>
            <span className="chip accent" title="主模型">
              {route.primary_model}
            </span>
            {route.fallback_models.map((fb) => (
              <span key={fb} className="chip" title="回退模型">
                回退 {fb}
              </span>
            ))}
          </li>
        ))}
      </ul>
    </div>
  );
}

export function AiSettingsScreen({ embedded = false }: { embedded?: boolean }) {
  const toast = useToast();
  const [settings, setSettings] = useState<AiSettings | null>(null);
  const [mode, setMode] = useState<Mode>("builtin");
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  const [fallbackModel, setFallbackModel] = useState("");
  const [routingPreset, setRoutingPreset] = useState<CustomRoutingPreset>("easy");
  const [routes, setRoutes] = useState<CustomTaskRoute[]>([]);
  const [planNotes, setPlanNotes] = useState<string[]>([]);
  const [discoveredModels, setDiscoveredModels] = useState<string[]>([]);
  const [apiKey, setApiKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [localCustom, setLocalCustom] = useState<LocalCustomAiSettings | null>(null);

  const modelOptions = useMemo(() => preferChatModelIds(discoveredModels), [discoveredModels]);

  const applySettings = (next: AiSettings) => {
    setSettings(next);
    setMode(next.mode);
    setBaseUrl(next.base_url ?? "");
    setModel(next.model ?? "");
  };

  useEffect(() => {
    let cancelled = false;
    const userId = apiClient.sessionUserId();
    void Promise.all([
      apiClient.getAiSettings(),
      userId ? loadLocalCustomAiSettings(userId) : Promise.resolve(null),
    ])
      .then(([next, local]) => {
        if (!cancelled) {
          setLocalCustom(local);
          applySettings(
            local
              ? {
                  ...next,
                  mode: "custom",
                  provider: "openai_compat",
                  base_url: local.baseUrl,
                  model: local.model,
                  configured: true,
                  key_mask: "本机安全存储",
                }
              : next,
          );
          if (local) {
            setBaseUrl(local.baseUrl);
            setModel(local.model);
            setFallbackModel(local.fallbackModel ?? "");
            setRoutingPreset(local.routingPreset);
            setRoutes(local.routes ?? []);
          }
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          const message = aiErrorMessage(error, "load");
          setErrorMessage(message);
          toast.show(message);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [toast]);

  const applyEasyPlan = (selected: string) => {
    const plan = buildEasyRoutePlan(selected);
    setRoutingPreset(plan.preset === "single" ? "easy" : plan.preset);
    setModel(plan.model);
    setFallbackModel(plan.fallback_model ?? "");
    setRoutes(plan.routes);
    setPlanNotes(plan.notes);
  };

  const ensureAdvancedRoutes = (primary: string): CustomTaskRoute[] => {
    if (routes.length > 0) return routes;
    return singleModelRoutes(primary || model || "model");
  };

  const save = async () => {
    setErrorMessage(null);
    setBusy(true);
    try {
      const userId = apiClient.sessionUserId();
      const effectiveKey = apiKey || localCustom?.apiKey || "";
      if (mode === "custom" && (!userId || !effectiveKey)) {
        throw new ApiError({
          code: "invalid_argument",
          status: 400,
          message: "API Key 不能为空",
        });
      }
      if (mode === "custom" && !model.trim()) {
        throw new ApiError({
          code: "invalid_argument",
          status: 400,
          message: "请选择或填写模型名称",
        });
      }
      const next = await apiClient.putAiSettings(
        mode === "custom"
          ? {
              mode,
              provider: "openai_compat",
              baseUrl,
              model: model.trim(),
            }
          : { mode },
      );
      if (mode === "custom" && userId) {
        const trimmed = model.trim();
        const resolvedRoutes =
          routingPreset === "single"
            ? undefined
            : routingPreset === "easy"
              ? singleModelRoutes(trimmed)
              : ensureAdvancedRoutes(trimmed).map((r) => ({
                  ...r,
                  primary_model: r.primary_model.trim() || trimmed,
                  fallback_models: r.fallback_models.map((f) => f.trim()).filter(Boolean),
                }));
        const local: LocalCustomAiSettings = {
          userId,
          baseUrl,
          model: trimmed,
          apiKey: effectiveKey,
          routingPreset,
          fallbackModel:
            routingPreset === "advanced" && fallbackModel.trim()
              ? fallbackModel.trim()
              : null,
          routes: resolvedRoutes,
        };
        await saveLocalCustomAiSettings(local);
        setLocalCustom(local);
        if (resolvedRoutes) setRoutes(resolvedRoutes);
      } else {
        await removeLocalCustomAiSettings();
        setLocalCustom(null);
        setRoutes([]);
        setPlanNotes([]);
        setDiscoveredModels([]);
      }
      setApiKey("");
      applySettings(
        mode === "custom" ? { ...next, configured: true, key_mask: "本机安全存储" } : next,
      );
      toast.show("AI 设置已保存");
    } catch (error: unknown) {
      const message = aiErrorMessage(error, "save");
      setErrorMessage(message);
      toast.show(message);
    } finally {
      setBusy(false);
    }
  };

  const test = async () => {
    setErrorMessage(null);
    setBusy(true);
    try {
      const effectiveKey = apiKey || localCustom?.apiKey || "";
      await apiClient.testAiSettings({
        provider: "openai_compat",
        baseUrl,
        model: model.trim() || "probe-model",
        apiKey: effectiveKey,
      });
      toast.show("连接正常");
    } catch (error: unknown) {
      const message = aiErrorMessage(error, "test");
      setErrorMessage(message);
      toast.show(message);
    } finally {
      setBusy(false);
    }
  };

  const discover = async (): Promise<string[]> => {
    const effectiveKey = apiKey || localCustom?.apiKey || "";
    if (!baseUrl.trim() || !effectiveKey) {
      throw new ApiError({
        code: "invalid_argument",
        status: 400,
        message: "请先填写 HTTPS 地址和 API Key",
      });
    }
    const { models } = await apiClient.discoverCustomModels({
      baseUrl,
      apiKey: effectiveKey,
    });
    const list = preferChatModelIds(models);
    setDiscoveredModels(list.length > 0 ? list : models);
    return list.length > 0 ? list : models;
  };

  const pullModels = async () => {
    setErrorMessage(null);
    setBusy(true);
    try {
      const list = await discover();
      if (list.length === 0) {
        toast.show("未返回模型列表，请手动填写模型名");
      } else {
        if (!model.trim() || !list.includes(model.trim())) {
          setModel(list[0]!);
        }
        toast.show(`已拉取 ${list.length} 个模型，请选择后保存或一键省心`);
      }
    } catch (error: unknown) {
      const message = aiErrorMessage(error, "discover");
      setErrorMessage(message);
      toast.show(message);
    } finally {
      setBusy(false);
    }
  };

  const easySetup = async () => {
    setErrorMessage(null);
    setBusy(true);
    try {
      let list = discoveredModels;
      if (list.length === 0) {
        list = await discover();
      }
      const selected = model.trim() || list[0] || "";
      if (!selected) {
        throw new ApiError({
          code: "invalid_argument",
          status: 400,
          message: "请先拉取并选择一个模型",
        });
      }
      if (list.length > 0 && !list.includes(selected) && !model.trim()) {
        // Prefer first discovered when input empty
      }
      applyEasyPlan(selected);
      setModel(selected);
      toast.show(`已将所有任务设为「${selected}」`);
    } catch (error: unknown) {
      const message = aiErrorMessage(error, "discover");
      setErrorMessage(message);
      toast.show(message);
    } finally {
      setBusy(false);
    }
  };

  const removeKey = async () => {
    try {
      await removeLocalCustomAiSettings();
      const next = await apiClient.deleteCustomAiKey();
      setApiKey("");
      setLocalCustom(null);
      setRoutes([]);
      setPlanNotes([]);
      setFallbackModel("");
      setDiscoveredModels([]);
      applySettings(next);
      toast.show("自定义密钥已删除");
    } catch (error: unknown) {
      const message = aiErrorMessage(error, "delete");
      setErrorMessage(message);
      toast.show(message);
    }
  };

  const updateAdvancedRoute = (
    task: string,
    patch: Partial<Pick<CustomTaskRoute, "primary_model" | "fallback_models">>,
  ) => {
    setRoutes((prev) => {
      const base = prev.length > 0 ? prev : singleModelRoutes(model || "model");
      return base.map((row) => (row.task === task ? { ...row, ...patch } : row));
    });
    setRoutingPreset("advanced");
  };

  if (!settings) {
    const loading = (
      <div className="panel">
        <h4>AI 设置</h4>
        <div className="skeleton" style={{ height: 180 }} />
      </div>
    );
    return embedded ? (
      loading
    ) : (
      <section className="settings">
        <h2 className="settings-title">AI 设置</h2>
        {loading}
      </section>
    );
  }

  const panel = (
    <div className="panel" aria-label="AI 设置">
      <h4>AI 设置</h4>
      <p className="cal-note">
        用于顶部「描述推荐」。桌面端密钥保存在系统凭据库；浏览器预览仅保留在当前标签页会话，服务端不会写入数据库。
      </p>
      <div className="seg" role="tablist" aria-label="AI 模式">
        {(["builtin", "custom", "off"] as const).map((choice) => (
          <button
            key={choice}
            type="button"
            className="btn small"
            aria-pressed={mode === choice}
            onClick={() => setMode(choice)}
          >
            {choice === "builtin" ? "内置 AI" : choice === "custom" ? "自定义 API" : "关闭 AI"}
          </button>
        ))}
      </div>

      {mode === "builtin" && (
        <>
          <div className="statusline ai-statusline">
            <span className={settings.builtin.available ? "chip ok" : "chip danger"}>
              {settings.builtin.available ? "可用" : "不可用"}
            </span>
            {settings.builtin.daily_remaining !== null && (
              <span className="chip">今日剩余 {settings.builtin.daily_remaining} 次</span>
            )}
          </div>
          <p className="cal-note">使用服务端内置 AI。模型与回退在后台自动完成，无需配置。</p>
        </>
      )}

      {mode === "custom" && (
        <div className="stack-form ai-custom-form">
          <p className="cal-note">
            OpenAI 兼容接口。可先<strong>拉取模型</strong>再选择；「一键省心」会把所有任务设为你选中的同一个模型（不按名称猜测能力）。会配置的人可改用高级分任务。
          </p>
          <label>
            HTTPS 地址
            <input
              type="url"
              value={baseUrl}
              placeholder="https://api.example.com/v1"
              onChange={(event) => setBaseUrl(event.target.value)}
            />
          </label>
          <label>
            API Key
            <input
              type="password"
              value={apiKey}
              autoComplete="off"
              onChange={(event) => setApiKey(event.target.value)}
              placeholder={settings.configured || localCustom ? "输入新密钥以替换" : ""}
            />
          </label>
          {(settings.configured || localCustom) && (
            <span className="chip ok">密钥 {settings.key_mask ?? "本机安全存储"}</span>
          )}

          <div className="seg">
            <button
              type="button"
              className="btn small"
              disabled={busy || (!apiKey && !localCustom) || !baseUrl.trim()}
              onClick={() => void pullModels()}
            >
              {busy ? "处理中" : "拉取模型列表"}
            </button>
            <button
              type="button"
              className="btn small"
              disabled={busy || (!apiKey && !localCustom)}
              onClick={() => void test()}
            >
              {apiKey ? "测试当前输入" : "测试已保存配置"}
            </button>
            {(settings.configured || localCustom) && (
              <button type="button" className="btn small ghost" onClick={() => void removeKey()}>
                删除密钥
              </button>
            )}
          </div>

          <label>
            模型
            {modelOptions.length > 0 ? (
              <select
                value={modelOptions.includes(model) ? model : ""}
                onChange={(event) => {
                  const next = event.target.value;
                  if (next) setModel(next);
                }}
                aria-label="从上游列表选择模型"
              >
                <option value="" disabled>
                  从列表选择…
                </option>
                {modelOptions.map((id) => (
                  <option key={id} value={id}>
                    {id}
                  </option>
                ))}
              </select>
            ) : null}
            <input
              value={model}
              placeholder={modelOptions.length > 0 ? "或手动输入模型名" : "例如 gpt-4o-mini"}
              onChange={(event) => setModel(event.target.value)}
              list={modelOptions.length > 0 ? "mpgs-discovered-models" : undefined}
            />
            {modelOptions.length > 0 && (
              <datalist id="mpgs-discovered-models">
                {modelOptions.map((id) => (
                  <option key={id} value={id} />
                ))}
              </datalist>
            )}
          </label>
          {modelOptions.length > 0 && (
            <p className="cal-note">已发现 {modelOptions.length} 个模型，可从下拉选择或手动填写。</p>
          )}

          <div className="easy-setup-box">
            <button
              type="button"
              className="btn accent"
              disabled={busy || (!apiKey && !localCustom) || !baseUrl.trim()}
              onClick={() => void easySetup()}
            >
              {busy ? "配置中…" : "一键省心"}
            </button>
            <p className="cal-note">
              拉取（或使用已选）模型后，将<strong>全部任务</strong>设为同一个模型。跨厂商不会根据名字猜「谁更快/更强」。
            </p>
          </div>

          <div className="seg" role="tablist" aria-label="自定义路由方式">
            <button
              type="button"
              className="btn small"
              aria-pressed={routingPreset === "easy"}
              onClick={() => {
                setRoutingPreset("easy");
                if (model.trim()) {
                  const plan = buildEasyRoutePlan(model);
                  setRoutes(plan.routes);
                  setPlanNotes(plan.notes);
                }
              }}
            >
              省心（统一模型）
            </button>
            <button
              type="button"
              className="btn small"
              aria-pressed={routingPreset === "advanced"}
              onClick={() => {
                setRoutingPreset("advanced");
                setRoutes(ensureAdvancedRoutes(model));
              }}
            >
              高级（分任务）
            </button>
            <button
              type="button"
              className="btn small"
              aria-pressed={routingPreset === "single"}
              onClick={() => {
                setRoutingPreset("single");
                setRoutes([]);
                setPlanNotes([]);
              }}
            >
              仅单模型请求
            </button>
          </div>

          {routingPreset === "advanced" && (
            <>
              <label>
                默认回退模型（可选，写入请求级 fallback）
                <input
                  value={fallbackModel}
                  placeholder="主模型失败时使用"
                  onChange={(event) => setFallbackModel(event.target.value)}
                  list={modelOptions.length > 0 ? "mpgs-discovered-models" : undefined}
                />
              </label>
              <div className="ai-routes" aria-label="分任务模型">
                <p className="cal-note">为每个任务指定主模型；回退可留空。需自行确认上游是否提供该模型。</p>
                <ul className="ai-route-list">
                  {(routes.length > 0 ? routes : singleModelRoutes(model || "model")).map((route) => (
                    <li key={route.task} className="ai-route-row ai-route-edit">
                      <strong title={route.task}>{taskLabel(route.task)}</strong>
                      <input
                        value={route.primary_model}
                        placeholder="主模型"
                        aria-label={`${taskLabel(route.task)} 主模型`}
                        list={modelOptions.length > 0 ? "mpgs-discovered-models" : undefined}
                        onChange={(event) =>
                          updateAdvancedRoute(route.task, { primary_model: event.target.value })
                        }
                      />
                      <input
                        value={route.fallback_models[0] ?? ""}
                        placeholder="回退（可选）"
                        aria-label={`${taskLabel(route.task)} 回退模型`}
                        list={modelOptions.length > 0 ? "mpgs-discovered-models" : undefined}
                        onChange={(event) => {
                          const fb = event.target.value.trim();
                          updateAdvancedRoute(route.task, {
                            fallback_models: fb ? [fb] : [],
                          });
                        }}
                      />
                    </li>
                  ))}
                </ul>
              </div>
            </>
          )}

          {routingPreset === "easy" && routes.length > 0 && (
            <RoutePreview
              title="省心路由预览"
              note={planNotes[0] ?? "所有任务使用同一模型。"}
              routes={routes}
            />
          )}

          {routingPreset === "single" && (
            <p className="cal-note">保存后描述推荐仅携带单一 model，不启用任务级路由表。</p>
          )}
        </div>
      )}

      {errorMessage && (
        <p className="cal-note ai-error" role="alert">
          {errorMessage}
        </p>
      )}
      <div className="onboarding-actions" style={{ justifyContent: "flex-start", marginTop: 16 }}>
        <button
          type="button"
          className="btn primary"
          disabled={busy || (mode === "custom" && !apiKey && !localCustom)}
          onClick={() => void save()}
        >
          {busy ? "处理中" : "保存"}
        </button>
      </div>
    </div>
  );

  if (embedded) return panel;
  return (
    <section className="settings ai-settings" aria-label="AI 设置">
      <h2 className="settings-title">AI 设置</h2>
      {panel}
    </section>
  );
}
