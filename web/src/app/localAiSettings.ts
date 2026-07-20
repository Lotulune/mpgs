import { invoke, isTauri } from "@tauri-apps/api/core";

const BROWSER_SESSION_KEY = "mpgs.ai.custom.session.v1";

export interface LocalCustomAiSettings {
  userId: string;
  baseUrl: string;
  model: string;
  apiKey: string;
}

function parse(value: string | null, userId: string): LocalCustomAiSettings | null {
  if (!value) return null;
  try {
    const parsed = JSON.parse(value) as Partial<LocalCustomAiSettings>;
    if (
      parsed.userId !== userId ||
      typeof parsed.baseUrl !== "string" ||
      typeof parsed.model !== "string" ||
      typeof parsed.apiKey !== "string" ||
      !parsed.apiKey
    ) {
      return null;
    }
    return parsed as LocalCustomAiSettings;
  } catch {
    return null;
  }
}

export async function loadLocalCustomAiSettings(
  userId: string,
): Promise<LocalCustomAiSettings | null> {
  const raw = isTauri()
    ? await invoke<string | null>("ai_credential_load")
    : globalThis.sessionStorage.getItem(BROWSER_SESSION_KEY);
  return parse(raw, userId);
}

export async function saveLocalCustomAiSettings(
  settings: LocalCustomAiSettings,
): Promise<void> {
  const value = JSON.stringify(settings);
  if (isTauri()) {
    await invoke("ai_credential_save", { value });
  } else {
    globalThis.sessionStorage.setItem(BROWSER_SESSION_KEY, value);
  }
}

export async function removeLocalCustomAiSettings(): Promise<void> {
  if (isTauri()) {
    await invoke("ai_credential_remove");
  } else {
    globalThis.sessionStorage.removeItem(BROWSER_SESSION_KEY);
  }
}
