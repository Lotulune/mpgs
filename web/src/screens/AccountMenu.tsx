import { useEffect, useRef, useState } from "react";
import type { AccountProfile } from "../api/types";
import { apiClient } from "../app/runtime";
import { useToast } from "../app/ToastProvider";

export function AccountMenu({
  profile,
  onLogin,
  onProfile,
  onAiSettings,
  onLogout,
}: {
  profile: AccountProfile | null;
  onLogin: () => void;
  onProfile: () => void;
  onAiSettings: () => void;
  onLogout: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [imageFailed, setImageFailed] = useState(false);
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const rootRef = useRef<HTMLDivElement>(null);

  // After a successful avatar change, reset the error latch so the new URL can render.
  useEffect(() => {
    setImageFailed(false);
  }, [profile?.avatar_url]);

  useEffect(() => {
    if (!open) return;
    const close = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [open]);

  if (!profile) {
    return (
      <button type="button" className="btn small" data-testid="auth-open-login" onClick={onLogin}>
        登录
      </button>
    );
  }

  const initial = profile.display_name.trim().slice(0, 1).toUpperCase() || "?";
  const logout = async (all: boolean) => {
    setOpen(false);
    setBusy(true);
    try {
      if (all) await apiClient.logoutAll();
      else await apiClient.logout();
      onLogout();
    } catch {
      toast.show("退出失败，请检查网络后重试");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="account-menu" ref={rootRef}>
      <button
        type="button"
        className="avatar-button"
        aria-label="账户菜单"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        {!imageFailed ? (
          <img src={profile.avatar_url} alt="" onError={() => setImageFailed(true)} />
        ) : (
          <span aria-hidden="true">{initial}</span>
        )}
      </button>
      {open && (
        <div className="account-popover" role="menu" aria-label="账户菜单">
          <div className="account-summary">
            <strong>{profile.display_name}</strong>
            <span>{profile.username}</span>
          </div>
          <button type="button" role="menuitem" onClick={() => { setOpen(false); onProfile(); }}>
            个人资料
          </button>
          <button type="button" role="menuitem" onClick={() => { setOpen(false); onAiSettings(); }}>
            AI 设置
          </button>
          <span className="menu-divider" />
          <button type="button" role="menuitem" disabled={busy} onClick={() => void logout(false)}>
            退出当前设备
          </button>
          <button type="button" role="menuitem" disabled={busy} onClick={() => void logout(true)}>
            退出全部设备
          </button>
        </div>
      )}
    </div>
  );
}
