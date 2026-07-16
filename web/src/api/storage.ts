import { invoke, isTauri } from "@tauri-apps/api/core";
import type { StorageLike } from "./types";

const MPGS_KEY_PREFIX = "mpgs.";

class SqliteBackedStorage implements StorageLike {
  private readonly values = new Map<string, string>();
  private writeChain: Promise<void> = Promise.resolve();
  private lastWriteError: unknown = null;

  hydrate(values: Record<string, string>): void {
    this.values.clear();
    for (const [key, value] of Object.entries(values)) this.values.set(key, value);
  }

  get length(): number {
    return this.values.size;
  }

  key(index: number): string | null {
    return Array.from(this.values.keys())[index] ?? null;
  }

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    const normalized = String(value);
    this.values.set(key, normalized);
    this.enqueue(() => invoke("client_store_set", { key, value: normalized }));
  }

  removeItem(key: string): void {
    this.values.delete(key);
    this.enqueue(() => invoke("client_store_remove", { key }));
  }

  private enqueue(write: () => Promise<unknown>): void {
    this.writeChain = this.writeChain
      .then(write)
      .then(() => undefined)
      .catch((error: unknown) => {
        // Keep later writes runnable. flush() still surfaces the last failure.
        this.lastWriteError = error;
      });
  }

  async flush(): Promise<void> {
    await this.writeChain;
    if (this.lastWriteError !== null) {
      const error = this.lastWriteError;
      this.lastWriteError = null;
      throw error;
    }
  }
}

let activeStorage: StorageLike | null = null;
let sqliteStorage: SqliteBackedStorage | null = null;

async function installDesktopCloseGuard(): Promise<void> {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const appWindow = getCurrentWindow();
  let closing = false;
  await appWindow.onCloseRequested((event) => {
    event.preventDefault();
    if (closing) return;
    closing = true;
    void flushClientStorage().finally(() => appWindow.destroy());
  });
}

/**
 * Hydrate the desktop key/value mirror before React and app singletons load.
 * Browser development keeps the native Web Storage fallback; packaged Tauri
 * builds persist all MPGS client state in the private application SQLite DB.
 */
export async function initializeClientStorage(): Promise<void> {
  if (!isTauri()) {
    activeStorage = globalThis.localStorage;
    return;
  }

  const store = new SqliteBackedStorage();
  const persisted = await invoke<Record<string, string>>("client_store_load");
  store.hydrate(persisted);

  // One-time migration for users of the previous localStorage-backed builds.
  const legacy = globalThis.localStorage;
  for (let index = 0; index < legacy.length; index += 1) {
    const key = legacy.key(index);
    if (!key?.startsWith(MPGS_KEY_PREFIX) || store.getItem(key) !== null) continue;
    const value = legacy.getItem(key);
    if (value !== null) store.setItem(key, value);
  }
  await store.flush();
  for (let index = legacy.length - 1; index >= 0; index -= 1) {
    const key = legacy.key(index);
    if (key?.startsWith(MPGS_KEY_PREFIX)) legacy.removeItem(key);
  }

  sqliteStorage = store;
  activeStorage = store;
  await installDesktopCloseGuard();
}

export function getClientStorage(): StorageLike {
  if (activeStorage) return activeStorage;
  if (!isTauri()) return globalThis.localStorage;
  throw new Error("desktop client storage was accessed before SQLite hydration");
}

/** Primarily useful before a controlled desktop shutdown or in persistence tests. */
export function flushClientStorage(): Promise<void> {
  return sqliteStorage?.flush() ?? Promise.resolve();
}
