// Typed HTTP client for the MPGS public API.
//
// Responsibilities:
// - anonymous session bootstrap + refresh-on-401 (single-flight)
// - ETag revalidation with a durable client snapshot cache (offline browsing)
// - stable error envelope parsing -> ApiError
// - x-device-id header for rate limiting fairness
//
// The client never touches server keys; everything here ships inside the desktop bundle.

import type {
  CalendarResponse,
  CalendarPeriod,
  ErrorEnvelope,
  EvidenceResponse,
  FeedbackRecord,
  FeedbackType,
  FeedResponse,
  FeedSection,
  GameDetail,
  MetaResponse,
  NaturalLanguageRecommendationResponse,
  PlayIntentResult,
  SearchResponse,
  SessionTokens,
  StorageLike,
  UserPreferences,
} from "./types";
import { getClientStorage } from "./storage";

const SESSION_KEY = "mpgs.session.v1";
const DEVICE_KEY = "mpgs.device.v1";
const CACHE_PREFIX = "mpgs.cache.v1:";

export type ApiErrorCode =
  | "invalid_argument"
  | "unauthenticated"
  | "forbidden"
  | "not_found"
  | "version_conflict"
  | "cursor_stale"
  | "unsupported_constraint"
  | "rate_limited"
  | "internal"
  | "temporarily_unavailable"
  | "network"
  | "unknown";

export class ApiError extends Error {
  readonly code: ApiErrorCode;
  readonly status: number;
  readonly requestId: string | null;
  /** True when the failure is connectivity-level, not a server verdict. */
  readonly offline: boolean;

  constructor(args: {
    code: ApiErrorCode;
    status: number;
    message: string;
    requestId?: string | null;
    offline?: boolean;
  }) {
    super(args.message);
    this.name = "ApiError";
    this.code = args.code;
    this.status = args.status;
    this.requestId = args.requestId ?? null;
    this.offline = args.offline ?? false;
  }
}

export interface CachedResult<T> {
  data: T;
  /** Unix ms when this payload was last confirmed fresh by the server. */
  fetchedAtMs: number;
  /** True when served from the local snapshot because the network failed. */
  fromOfflineCache: boolean;
}

interface CacheEntry<T> {
  etag: string | null;
  fetchedAtMs: number;
  data: T;
}

export interface ApiClientOptions {
  baseUrl?: string;
  fetchFn?: typeof fetch;
  storage?: StorageLike;
  now?: () => number;
}

function randomId(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchFn: typeof fetch;
  private readonly storage: StorageLike;
  private readonly now: () => number;
  private session: SessionTokens | null = null;
  private sessionPromise: Promise<SessionTokens | null> | null = null;

  constructor(options: ApiClientOptions = {}) {
    this.baseUrl = (options.baseUrl ?? "").replace(/\/$/, "");
    this.fetchFn = options.fetchFn ?? fetch.bind(globalThis);
    this.storage = options.storage ?? getClientStorage();
    this.now = options.now ?? Date.now;
    this.session = this.loadSession();
  }

  // --- device / session persistence ---

  deviceId(): string {
    let id = this.storage.getItem(DEVICE_KEY);
    if (!id) {
      id = `dev-${randomId()}`;
      this.storage.setItem(DEVICE_KEY, id);
    }
    return id;
  }

  private loadSession(): SessionTokens | null {
    try {
      const raw = this.storage.getItem(SESSION_KEY);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as SessionTokens;
      if (typeof parsed.access_token !== "string" || typeof parsed.refresh_token !== "string") {
        return null;
      }
      return parsed;
    } catch {
      return null;
    }
  }

  private saveSession(session: SessionTokens | null): void {
    this.session = session;
    if (session) {
      this.storage.setItem(SESSION_KEY, JSON.stringify(session));
    } else {
      this.storage.removeItem(SESSION_KEY);
    }
  }

  /** Mark the access token expired but keep the refresh token for a refresh. */
  private invalidateAccess(): void {
    if (this.session) {
      this.saveSession({ ...this.session, expires_at_ms: 0 });
    }
  }

  hasSession(): boolean {
    return this.session !== null;
  }

  /** Current anonymous identity, used to scope persisted user-specific state. */
  sessionUserId(): string | null {
    return this.session?.user_id ?? null;
  }

  /**
   * Ensure a usable anonymous session. Refreshes an expired access token via the
   * refresh token, creating a brand-new anonymous session as a last resort.
   * Single-flight: concurrent callers share one bootstrap.
   */
  async ensureSession(): Promise<SessionTokens | null> {
    if (this.session && this.session.expires_at_ms > this.now() + 30_000) {
      return this.session;
    }
    this.sessionPromise ??= this.bootstrapSession().finally(() => {
      this.sessionPromise = null;
    });
    return this.sessionPromise;
  }

  private async bootstrapSession(): Promise<SessionTokens | null> {
    const current = this.session;
    if (current && current.refresh_expires_at_ms > this.now() + 30_000) {
      try {
        const refreshed = await this.rawJson<SessionTokens>("POST", "/v1/session/refresh", {
          body: { refresh_token: current.refresh_token },
          auth: false,
        });
        this.saveSession(refreshed);
        return refreshed;
      } catch (error) {
        // A rejected refresh token cannot recover this identity. Transient
        // server and network failures must preserve it so a later retry can.
        if (!(error instanceof ApiError && error.status === 401)) throw error;
      }
    }
    const fresh = await this.rawJson<SessionTokens>("POST", "/v1/session/anonymous", {
      auth: false,
    });
    this.saveSession(fresh);
    return fresh;
  }

  // --- low level ---

  private async rawResponse(
    method: string,
    path: string,
    args: {
      body?: unknown;
      auth?: boolean;
      headers?: Record<string, string>;
    } = {},
  ): Promise<Response> {
    const headers: Record<string, string> = {
      "x-device-id": this.deviceId(),
      ...args.headers,
    };
    if (args.body !== undefined) {
      headers["content-type"] = "application/json";
    }
    if (args.auth) {
      const session = await this.ensureSession();
      if (session) {
        headers.authorization = `Bearer ${session.access_token}`;
      }
    }
    let response: Response;
    try {
      response = await this.fetchFn(`${this.baseUrl}${path}`, {
        method,
        headers,
        body: args.body === undefined ? null : JSON.stringify(args.body),
      });
    } catch (cause) {
      throw new ApiError({
        code: "network",
        status: 0,
        message: cause instanceof Error ? cause.message : "network request failed",
        offline: true,
      });
    }
    if (response.status === 401 && args.auth) {
      // Access token rejected: refresh (keeping the refresh token) and retry once.
      this.invalidateAccess();
      const session = await this.ensureSession();
      if (session) {
        headers.authorization = `Bearer ${session.access_token}`;
        try {
          response = await this.fetchFn(`${this.baseUrl}${path}`, {
            method,
            headers,
            body: args.body === undefined ? null : JSON.stringify(args.body),
          });
        } catch (cause) {
          throw new ApiError({
            code: "network",
            status: 0,
            message: cause instanceof Error ? cause.message : "network request failed",
            offline: true,
          });
        }
      }
    }
    return response;
  }

  private async parseError(response: Response): Promise<ApiError> {
    let code: ApiErrorCode = "unknown";
    let message = `HTTP ${response.status}`;
    let requestId: string | null = response.headers.get("x-request-id");
    try {
      const body = (await response.json()) as ErrorEnvelope;
      if (body && typeof body.error?.code === "string") {
        code = body.error.code as ApiErrorCode;
        message = body.error.message ?? message;
        requestId = body.error.request_id ?? requestId;
      }
    } catch {
      // keep defaults; error body is optional
    }
    return new ApiError({ code, status: response.status, message, requestId });
  }

  private async rawJson<T>(
    method: string,
    path: string,
    args: { body?: unknown; auth?: boolean; headers?: Record<string, string> } = {},
  ): Promise<T> {
    const response = await this.rawResponse(method, path, args);
    if (!response.ok) {
      throw await this.parseError(response);
    }
    return (await response.json()) as T;
  }

  // --- ETag snapshot cache ---

  private readCache<T>(key: string): CacheEntry<T> | null {
    try {
      const raw = this.storage.getItem(CACHE_PREFIX + key);
      if (!raw) return null;
      return JSON.parse(raw) as CacheEntry<T>;
    } catch {
      return null;
    }
  }

  private writeCache<T>(key: string, entry: CacheEntry<T>): void {
    try {
      this.storage.setItem(CACHE_PREFIX + key, JSON.stringify(entry));
    } catch {
      // Quota errors must never break the UI; stale cache is acceptable.
    }
  }

  /**
   * Remove every cached response snapshot. Session, device id and the pending
   * feedback queue live under different keys and are intentionally preserved
   * (clearing cache must never drop unsynced feedback).
   */
  clearCachedResponses(): number {
    const store = this.storage as Partial<Storage>;
    if (typeof store.length !== "number" || typeof store.key !== "function") {
      return 0; // storage without enumeration (e.g. minimal test doubles)
    }
    const keys: string[] = [];
    for (let i = 0; i < store.length; i += 1) {
      const key = store.key(i);
      if (key && key.startsWith(CACHE_PREFIX)) keys.push(key);
    }
    for (const key of keys) this.storage.removeItem(key);
    return keys.length;
  }

  /**
   * GET with ETag revalidation backed by the snapshot cache.
   * - 200: store payload + etag, return fresh data
   * - 304: refresh timestamp, return cached data
   * - network failure: return cached data flagged fromOfflineCache, else rethrow
   */
  private async cachedGet<T>(key: string, path: string, auth: boolean): Promise<CachedResult<T>> {
    const cached = this.readCache<T>(key);
    const headers: Record<string, string> = {};
    if (cached?.etag) {
      headers["if-none-match"] = cached.etag;
    }
    let response: Response;
    try {
      response = await this.rawResponse("GET", path, { auth, headers });
    } catch (error) {
      if (error instanceof ApiError && error.offline && cached) {
        return { data: cached.data, fetchedAtMs: cached.fetchedAtMs, fromOfflineCache: true };
      }
      throw error;
    }
    if (response.status === 304 && cached) {
      const entry: CacheEntry<T> = { ...cached, fetchedAtMs: this.now() };
      this.writeCache(key, entry);
      return { data: cached.data, fetchedAtMs: entry.fetchedAtMs, fromOfflineCache: false };
    }
    if (!response.ok) {
      throw await this.parseError(response);
    }
    const data = (await response.json()) as T;
    const entry: CacheEntry<T> = {
      etag: response.headers.get("etag"),
      fetchedAtMs: this.now(),
      data,
    };
    this.writeCache(key, entry);
    return { data, fetchedAtMs: entry.fetchedAtMs, fromOfflineCache: false };
  }

  // --- public endpoints ---

  meta(): Promise<CachedResult<MetaResponse>> {
    return this.cachedGet<MetaResponse>("meta", "/v1/meta", false);
  }

  feed(
    section: FeedSection,
    query: {
      limit?: number;
      cursor?: string;
      partySize?: number;
      demoOnly?: boolean;
    } = {},
  ): Promise<CachedResult<FeedResponse>> {
    const params = new URLSearchParams();
    if (query.limit) params.set("limit", String(query.limit));
    if (query.cursor) params.set("cursor", query.cursor);
    if (query.partySize) params.set("party_size", String(query.partySize));
    if (query.demoOnly) params.set("demo_only", "true");
    const qs = params.toString();
    const path = `/v1/feeds/${section}${qs ? `?${qs}` : ""}`;
    // Cursor pages are not cached: only first pages serve as offline snapshots.
    if (query.cursor) {
      return this.uncachedGet<FeedResponse>(path, this.hasSession());
    }
    const cacheKey = `feed:${section}:${query.partySize ?? "p"}:${query.demoOnly ? 1 : 0}:${
      this.session?.user_id ?? "anon"
    }`;
    return this.cachedGet<FeedResponse>(cacheKey, path, this.hasSession());
  }

  private async uncachedGet<T>(path: string, auth: boolean): Promise<CachedResult<T>> {
    const data = await this.rawJson<T>("GET", path, { auth });
    return { data, fetchedAtMs: this.now(), fromOfflineCache: false };
  }

  calendar(
    fromDay: string,
    toDay: string,
    period: CalendarPeriod = "upcoming",
  ): Promise<CachedResult<CalendarResponse>> {
    const params = new URLSearchParams({ from: fromDay, to: toDay, state: period });
    return this.cachedGet<CalendarResponse>(
      `calendar:${period}:${fromDay}:${toDay}`,
      `/v1/calendar?${params}`,
      false,
    );
  }

  async search(q: string, limit = 20): Promise<SearchResponse> {
    const params = new URLSearchParams({ q, limit: String(limit) });
    return this.rawJson<SearchResponse>("GET", `/v1/search?${params}`, { auth: false });
  }

  async naturalLanguageRecommendations(
    query: string,
    limit = 6,
  ): Promise<NaturalLanguageRecommendationResponse> {
    return this.rawJson<NaturalLanguageRecommendationResponse>(
      "POST",
      "/v1/recommendations/natural-language",
      { auth: true, body: { query, limit } },
    );
  }

  game(appId: number): Promise<CachedResult<GameDetail>> {
    // Authenticated when a session exists so the response carries this user's
    // play-intent vote state; falls back to anonymous (voted always false).
    return this.cachedGet<GameDetail>(
      `game:${appId}:${this.session?.user_id ?? "anon"}`,
      `/v1/games/${appId}`,
      this.hasSession(),
    );
  }

  evidence(appId: number, feature?: string): Promise<CachedResult<EvidenceResponse>> {
    const qs = feature ? `?feature=${encodeURIComponent(feature)}` : "";
    return this.cachedGet<EvidenceResponse>(
      `evidence:${appId}:${feature ?? "all"}`,
      `/v1/games/${appId}/evidence${qs}`,
      false,
    );
  }

  async getPreferences(): Promise<UserPreferences> {
    return this.rawJson<UserPreferences>("GET", "/v1/preferences", { auth: true });
  }

  async putPreferences(prefs: UserPreferences): Promise<UserPreferences> {
    return this.rawJson<UserPreferences>("PUT", "/v1/preferences", {
      auth: true,
      body: prefs,
    });
  }

  async postFeedback(args: {
    appId: number;
    type: FeedbackType;
    idempotencyKey: string;
    clientCreatedAtMs: number;
  }): Promise<FeedbackRecord> {
    return this.rawJson<FeedbackRecord>("POST", "/v1/feedback", {
      auth: true,
      headers: { "idempotency-key": args.idempotencyKey },
      body: {
        app_id: args.appId,
        type: args.type,
        client_created_at_ms: args.clientCreatedAtMs,
      },
    });
  }

  async undoFeedback(feedbackId: number): Promise<FeedbackRecord> {
    return this.rawJson<FeedbackRecord>("POST", `/v1/feedback/${feedbackId}/undo`, {
      auth: true,
    });
  }

  async setPlayIntent(appId: number, intent: boolean): Promise<PlayIntentResult> {
    return this.rawJson<PlayIntentResult>("POST", `/v1/games/${appId}/play-intent`, {
      auth: true,
      body: { intent },
    });
  }
}

export function newIdempotencyKey(): string {
  return `idem-${randomId()}`;
}
