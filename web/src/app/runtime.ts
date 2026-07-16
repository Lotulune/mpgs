// App-level singletons and light global state (no external state library).

import { ApiClient } from "../api/client";
import { FeedbackQueue } from "../api/feedbackQueue";
import { PlayIntentStore } from "../api/playIntentStore";
import { getClientStorage } from "../api/storage";
import type { StorageLike } from "../api/types";
import { flushPendingPreferencePatch } from "./preferences";

// Dev + Tauri-dev load through the Vite proxy, so same-origin ("") works.
// A packaged build is served from the webview origin and must call the server
// absolutely; the server's CORS allowlist covers that origin.
const API_BASE =
  import.meta.env.VITE_MPGS_API_BASE ??
  (import.meta.env.PROD ? "http://127.0.0.1:8080" : "");

const storage = getClientStorage();
export const apiClient = new ApiClient({ baseUrl: API_BASE, storage });
export const feedbackQueue = new FeedbackQueue(apiClient, storage);
export const playIntentStore = new PlayIntentStore(apiClient, storage);

// Replay pending feedback and votes when connectivity returns.
if (typeof window !== "undefined") {
  void feedbackQueue.flush();
  void playIntentStore.flush();
  void flushPendingPreferencePatch(apiClient).catch(() => undefined);
  window.addEventListener("online", () => {
    void feedbackQueue.flush();
    void playIntentStore.flush();
    void flushPendingPreferencePatch(apiClient).catch(() => undefined);
  });
}

const ONBOARDED_KEY = "mpgs.onboarded.v1";

export function isOnboarded(storage: StorageLike = getClientStorage()): boolean {
  try {
    return storage.getItem(ONBOARDED_KEY) === "true";
  } catch {
    return false;
  }
}

export function markOnboarded(storage: StorageLike = getClientStorage()): void {
  try {
    storage.setItem(ONBOARDED_KEY, "true");
  } catch {
    // best effort
  }
}

const FX_KEY = "mpgs.fx.v1";

export function loadFxIntensity(storage: StorageLike = getClientStorage()): string | null {
  try {
    return storage.getItem(FX_KEY);
  } catch {
    return null;
  }
}

export function saveFxIntensity(value: string, storage: StorageLike = getClientStorage()): void {
  try {
    storage.setItem(FX_KEY, value);
  } catch {
    // best effort
  }
}
