// Shared test helpers: in-memory Storage and a scriptable fetch stub.

import type { SessionTokens, StorageLike } from "../src/api/types";

export class MemoryStorage implements StorageLike {
  private map = new Map<string, string>();

  // length/key mirror the DOM Storage surface so cache enumeration can be tested.
  get length(): number {
    return this.map.size;
  }

  key(index: number): string | null {
    return Array.from(this.map.keys())[index] ?? null;
  }

  clear(): void {
    this.map.clear();
  }

  getItem(key: string): string | null {
    return this.map.has(key) ? (this.map.get(key) as string) : null;
  }

  removeItem(key: string): void {
    this.map.delete(key);
  }

  setItem(key: string, value: string): void {
    this.map.set(key, String(value));
  }
}

export interface StubCall {
  url: string;
  method: string;
  headers: Record<string, string>;
  body: unknown;
}

export type StubHandler = (call: StubCall) => Response | Promise<Response>;

/** Builds a fetch stub that dispatches on `METHOD path` keys, recording calls. */
export function makeFetchStub(routes: Record<string, StubHandler>) {
  const calls: StubCall[] = [];
  const fetchFn = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === "string" ? input : input.toString();
    const method = (init?.method ?? "GET").toUpperCase();
    const headers: Record<string, string> = {};
    const rawHeaders = init?.headers as Record<string, string> | undefined;
    if (rawHeaders) {
      for (const [k, v] of Object.entries(rawHeaders)) headers[k.toLowerCase()] = v;
    }
    const body = init?.body ? JSON.parse(init.body as string) : undefined;
    const call: StubCall = { url, method, headers, body };
    calls.push(call);
    const path = url.replace(/^https?:\/\/[^/]+/, "");
    const key = `${method} ${path.split("?")[0]}`;
    const handler = routes[key] ?? routes[`${method} ${path}`];
    if (!handler) {
      return new Response(JSON.stringify({ error: { code: "not_found", message: "no stub" } }), {
        status: 404,
        headers: { "content-type": "application/json" },
      });
    }
    return handler(call);
  }) as typeof fetch;
  return { fetchFn, calls };
}

export function jsonResponse(body: unknown, init: ResponseInit = {}): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    ...init,
    headers: { "content-type": "application/json", ...(init.headers ?? {}) },
  });
}

export function sessionBody(overrides: Partial<SessionTokens> = {}): SessionTokens {
  return {
    access_token: "access-1",
    refresh_token: "refresh-1",
    user_id: "u_test",
    account: true,
    expires_at_ms: Date.now() + 3_600_000,
    refresh_expires_at_ms: Date.now() + 30 * 24 * 3_600_000,
    ...overrides,
  };
}

export function seedAccountSession(
  storage: StorageLike,
  overrides: Partial<SessionTokens> = {},
): void {
  storage.setItem("mpgs.session.v1", JSON.stringify(sessionBody({ account: true, ...overrides })));
}
