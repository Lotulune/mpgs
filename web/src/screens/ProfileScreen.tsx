// Styles: styles/screens/settings.css（.profile-screen 作用域）+ base.css 共享类。

import { useState } from "react";
import type { ChangeEvent, FormEvent } from "react";
import { ApiError } from "../api/client";
import type { AccountProfile } from "../api/types";
import { apiClient } from "../app/runtime";
import { useToast } from "../app/ToastProvider";
import { Button } from "../components/Button";
import { Panel } from "../components/Panel";
import { Avatar } from "../components/Avatar";

export function ProfileScreen({
  profile,
  onUpdated,
  onDeleted,
}: {
  profile: AccountProfile;
  onUpdated: (profile: AccountProfile) => void;
  onDeleted: () => void;
}) {
  const toast = useToast();
  const [displayName, setDisplayName] = useState(profile.display_name);
  const [oldPassword, setOldPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [saving, setSaving] = useState(false);

  const saveName = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    try {
      const updated = await apiClient.updateMe(displayName);
      onUpdated(updated);
      toast.show("资料已更新");
    } catch {
      toast.show("无法更新资料");
    } finally {
      setSaving(false);
    }
  };

  const upload = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    if (file.size > 2 * 1024 * 1024) {
      toast.show("头像不能超过 2 MiB");
      return;
    }
    try {
      const updated = await apiClient.uploadAvatar(file);
      onUpdated(updated);
      toast.show("头像已更新");
    } catch (error) {
      if (error instanceof ApiError) {
        if (error.code === "unauthenticated") {
          toast.show("请先登录后再更换头像");
          return;
        }
        if (error.code === "invalid_avatar" || error.status === 400) {
          toast.show(error.message || "图片格式无效，请使用 JPEG / PNG / WebP");
          return;
        }
        toast.show(error.message || "头像上传失败");
        return;
      }
      toast.show("头像上传失败");
    }
  };

  const savePassword = async (event: FormEvent) => {
    event.preventDefault();
    try {
      await apiClient.changePassword(oldPassword, newPassword);
      setOldPassword("");
      setNewPassword("");
      toast.show("密码已更新");
    } catch {
      toast.show("无法更新密码");
    }
  };

  const removeAccount = async () => {
    if (!window.confirm("确认注销此账户？")) return;
    try {
      await apiClient.deleteMe();
      toast.show("账户已注销");
      onDeleted();
    } catch {
      toast.show("账户注销失败");
    }
  };

  const removeAvatar = async () => {
    try {
      await apiClient.deleteAvatar();
      onUpdated(await apiClient.getMe());
      toast.show("已恢复默认头像");
    } catch {
      toast.show("无法删除头像");
    }
  };

  return (
    <section className="settings profile-screen" aria-label="个人资料">
      <h2 className="settings-title">个人资料</h2>

      <Panel title="头像" className="profile-identity-panel">
        <div className="profile-layout">
          <div className="profile-avatar-large">
            <Avatar
              key={profile.avatar_url}
              src={profile.avatar_url}
              name={profile.display_name}
              alt={`${profile.display_name} 的头像`}
            />
          </div>
          <div className="profile-identity">
            <strong>{profile.display_name}</strong>
            <span>{profile.username}</span>
            <div className="profile-avatar-actions">
              <label className="btn small ghost file-button">
                更换头像
                <input
                  type="file"
                  accept="image/jpeg,image/png,image/webp"
                  onChange={(event) => void upload(event)}
                />
              </label>
              <Button size="small" variant="ghost" onClick={() => void removeAvatar()}>
                使用默认头像
              </Button>
            </div>
            <p className="cal-note settings-note">支持 JPEG / PNG / WebP，不超过 2 MiB。</p>
          </div>
        </div>
      </Panel>

      <Panel
        as="form"
        title="公开资料"
        className="stack-form"
        onSubmit={(event) => void saveName(event)}
      >
        <label>
          显示名称
          <input
            value={displayName}
            maxLength={40}
            onChange={(event) => setDisplayName(event.target.value)}
            required
          />
        </label>
        <div className="settings-actions">
          <Button type="submit" variant="primary" disabled={saving}>
            保存
          </Button>
        </div>
      </Panel>

      <Panel
        as="form"
        title="密码"
        className="stack-form"
        onSubmit={(event) => void savePassword(event)}
      >
        <label>
          当前密码
          <input
            type="password"
            value={oldPassword}
            autoComplete="current-password"
            onChange={(event) => setOldPassword(event.target.value)}
            required
          />
        </label>
        <label>
          新密码
          <input
            type="password"
            value={newPassword}
            minLength={10}
            maxLength={128}
            autoComplete="new-password"
            onChange={(event) => setNewPassword(event.target.value)}
            required
          />
        </label>
        <div className="settings-actions">
          <Button type="submit" variant="primary">
            更新密码
          </Button>
        </div>
      </Panel>

      <Panel title="危险操作" className="danger-zone">
        <p className="danger-note">
          注销账户将删除云端资料与偏好，且无法撤销。请确认不再需要该账户后再操作。
        </p>
        <Button variant="danger" onClick={() => void removeAccount()}>
          注销账户
        </Button>
      </Panel>
    </section>
  );
}
