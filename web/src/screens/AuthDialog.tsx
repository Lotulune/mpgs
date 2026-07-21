import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { ApiError } from "../api/client";
import { apiClient } from "../app/runtime";

type Mode = "login" | "register";

export function AuthDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<Mode>("login");
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [mergePreference, setMergePreference] = useState<"anonymous" | "account" | undefined>();
  const [mergeRequired, setMergeRequired] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const usernameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    setError(null);
    setBusy(false);
    queueMicrotask(() => usernameRef.current?.focus());
  }, [open]);

  if (!open) return null;

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      if (mode === "register") {
        await apiClient.register({ username, displayName, password });
      } else {
        await apiClient.login({ username, password, mergePreference });
      }
      setPassword("");
      onClose();
    } catch (cause) {
      if (cause instanceof ApiError && cause.code === "merge_choice_required") {
        setMergeRequired(true);
        setError("请选择要保留的偏好设置。");
      } else if (cause instanceof ApiError && cause.code === "account_conflict") {
        setError("该用户名不可用。");
      } else if (cause instanceof ApiError && cause.code === "rate_limited") {
        setError("尝试次数过多，请稍后再试。");
      } else {
        setError("无法完成登录，请检查输入后重试。");
      }
    } finally {
      setBusy(false);
    }
  };

  const switchMode = (next: Mode) => {
    setMode(next);
    setError(null);
    setMergeRequired(false);
    setMergePreference(undefined);
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="modal auth-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="auth-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="modal-head">
          <h2 id="auth-title">{mode === "login" ? "登录" : "注册"}</h2>
          <button type="button" className="icon-btn" aria-label="关闭" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="seg auth-mode" role="tablist" aria-label="账户操作">
          <button
            type="button"
            className="btn small"
            data-testid="auth-mode-login"
            aria-pressed={mode === "login"}
            onClick={() => switchMode("login")}
          >
            登录
          </button>
          <button
            type="button"
            className="btn small"
            data-testid="auth-mode-register"
            aria-pressed={mode === "register"}
            onClick={() => switchMode("register")}
          >
            注册
          </button>
        </div>
        <form className="stack-form" onSubmit={(event) => void submit(event)}>
          <label>
            用户名
            <input
              ref={usernameRef}
              data-testid="auth-username"
              value={username}
              minLength={3}
              maxLength={32}
              pattern="[A-Za-z0-9_]+"
              autoComplete="username"
              onChange={(event) => setUsername(event.target.value)}
              required
            />
          </label>
          {mode === "register" && (
            <label>
              显示名称
              <input
                data-testid="auth-display-name"
                value={displayName}
                maxLength={40}
                autoComplete="nickname"
                onChange={(event) => setDisplayName(event.target.value)}
                required
              />
            </label>
          )}
          <label>
            密码
            <input
              type="password"
              data-testid="auth-password"
              value={password}
              minLength={10}
              maxLength={128}
              autoComplete={mode === "login" ? "current-password" : "new-password"}
              onChange={(event) => setPassword(event.target.value)}
              required
            />
          </label>
          {mergeRequired && mode === "login" && (
            <fieldset className="choice-fieldset">
              <legend>偏好设置</legend>
              <label>
                <input
                  type="radio"
                  name="merge-preference"
                  checked={mergePreference === "account"}
                  onChange={() => setMergePreference("account")}
                />
                保留账户偏好
              </label>
              <label>
                <input
                  type="radio"
                  name="merge-preference"
                  checked={mergePreference === "anonymous"}
                  onChange={() => setMergePreference("anonymous")}
                />
                保留本机偏好
              </label>
            </fieldset>
          )}
          {error && <p className="form-error" role="alert">{error}</p>}
          <button
            type="submit"
            className="btn primary"
            data-testid="auth-submit"
            disabled={busy || (mergeRequired && !mergePreference)}
          >
            {busy ? "处理中" : mode === "login" ? "登录" : "创建账户"}
          </button>
        </form>
      </section>
    </div>
  );
}
