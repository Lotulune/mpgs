import { describe, expect, it } from "vitest";
import { ApiClient } from "../src/api/client";
import { PlayIntentStore } from "../src/api/playIntentStore";
import { jsonResponse, makeFetchStub, MemoryStorage, seedAccountSession, sessionBody } from "./helpers";

function accountClient(storage: MemoryStorage, fetchFn: typeof fetch) {
  if (!storage.getItem("mpgs.session.v1")) seedAccountSession(storage);
  return new ApiClient({ baseUrl: "http://x", fetchFn, storage });
}

function makeClient(storage: MemoryStorage, online: () => boolean) {
  const { fetchFn, calls } = makeFetchStub({
    "POST /v1/session/anonymous": () => jsonResponse(sessionBody()),
    "POST /v1/games/10/play-intent": (call) => {
      if (!online()) throw new TypeError("offline");
      const intent = (call.body as { intent: boolean }).intent;
      return jsonResponse({ app_id: 10, count: intent ? 1 : 0, voted: intent });
    },
  });
  const client = accountClient(storage, fetchFn);
  return { client, calls };
}

describe("PlayIntentStore", () => {
  it("optimistically toggles and syncs", async () => {
    const storage = new MemoryStorage();
    const { client } = makeClient(storage, () => true);
    const store = new PlayIntentStore(client, storage);

    store.toggle(10, false);
    expect(store.effectiveVoted(10, false)).toBe(true);
    expect(store.countDelta(10, false)).toBe(1);
    expect(store.countDelta(10, true)).toBe(0); // already reflected server-side
    expect(store.isPending(10)).toBe(true);

    await store.flush();
    expect(store.isPending(10)).toBe(false);
    expect(store.effectiveVoted(10, false)).toBe(true);
  });

  it("un-voting produces a negative delta against a server 'voted' baseline", () => {
    const storage = new MemoryStorage();
    const { client } = makeClient(storage, () => true);
    const store = new PlayIntentStore(client, storage);
    // server says voted=true; user toggles off
    store.toggle(10, true);
    expect(store.effectiveVoted(10, true)).toBe(false);
    expect(store.countDelta(10, true)).toBe(-1);
  });

  it("keeps an offline vote pending and replays it across a reload", async () => {
    const storage = new MemoryStorage();
    let online = false;
    const a = makeClient(storage, () => online);
    const storeA = new PlayIntentStore(a.client, storage);
    storeA.toggle(10, false);
    await storeA.flush();
    expect(storeA.isPending(10)).toBe(true);

    online = true;
    const b = makeClient(storage, () => online);
    const storeB = new PlayIntentStore(b.client, storage);
    expect(storeB.effectiveVoted(10, false)).toBe(true); // restored from storage
    expect(storeB.isPending(10)).toBe(true);
    await storeB.flush();
    expect(storeB.isPending(10)).toBe(false);
  });

  it("reverts the optimistic override on a permanent error", async () => {
    const storage = new MemoryStorage();
    const { fetchFn } = makeFetchStub({
      "POST /v1/session/anonymous": () => jsonResponse(sessionBody()),
      "POST /v1/games/10/play-intent": () =>
        jsonResponse({ error: { code: "not_found", message: "x" } }, { status: 404 }),
    });
    const client = accountClient(storage, fetchFn);
    const store = new PlayIntentStore(client, storage);
    store.toggle(10, false);
    await store.flush();
    expect(store.effectiveVoted(10, false)).toBe(false);
    expect(store.isPending(10)).toBe(false);
  });

  it("serializes rapid toggles and leaves the latest intent authoritative", async () => {
    const storage = new MemoryStorage();
    let resolveFirst!: (response: Response) => void;
    let firstStarted!: () => void;
    const started = new Promise<void>((resolve) => {
      firstStarted = resolve;
    });
    const firstResponse = new Promise<Response>((resolve) => {
      resolveFirst = resolve;
    });
    let attempts = 0;
    const { fetchFn, calls } = makeFetchStub({
      "POST /v1/session/anonymous": () => jsonResponse(sessionBody()),
      "POST /v1/games/10/play-intent": (call) => {
        attempts += 1;
        if (attempts === 1) {
          firstStarted();
          return firstResponse;
        }
        const intent = (call.body as { intent: boolean }).intent;
        return jsonResponse({ app_id: 10, count: intent ? 1 : 0, voted: intent });
      },
    });
    const store = new PlayIntentStore(accountClient(storage, fetchFn), storage);
    store.toggle(10, false);
    await started;
    store.toggle(10, false);
    expect(store.effectiveVoted(10, false)).toBe(false);

    resolveFirst(jsonResponse({ app_id: 10, count: 1, voted: true }));
    await store.flush();

    const intents = calls
      .filter((call) => call.url.endsWith("/play-intent"))
      .map((call) => (call.body as { intent: boolean }).intent);
    expect(intents).toEqual([true, false]);
    expect(store.effectiveVoted(10, false)).toBe(false);
    expect(store.isPending(10)).toBe(false);
  });

  it("drops a settled override once the server baseline reflects it", async () => {
    const storage = new MemoryStorage();
    const { client } = makeClient(storage, () => true);
    const store = new PlayIntentStore(client, storage);
    store.toggle(10, false);
    await store.flush();

    expect(store.effectiveVoted(10, true)).toBe(true);
    expect(store.effectiveVoted(10, false)).toBe(false);
  });

  it("does not apply a settled override to a different identity", async () => {
    const storage = new MemoryStorage();
    storage.setItem("mpgs.session.v1", JSON.stringify(sessionBody({ user_id: "u_one" })));
    const first = makeClient(storage, () => true);
    const storeA = new PlayIntentStore(first.client, storage);
    storeA.toggle(10, false);
    await storeA.flush();
    expect(storeA.effectiveVoted(10, false)).toBe(true);

    storage.setItem("mpgs.session.v1", JSON.stringify(sessionBody({ user_id: "u_two" })));
    const second = makeClient(storage, () => true);
    const storeB = new PlayIntentStore(second.client, storage);
    expect(storeB.effectiveVoted(10, false)).toBe(false);
  });
});
