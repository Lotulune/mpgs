import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

const runtime = vi.hoisted(() => ({ search: vi.fn() }));

vi.mock("../src/app/runtime", () => ({
  apiClient: { search: runtime.search },
}));

import { SearchScreen } from "../src/screens/SearchScreen";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("SearchScreen", () => {
  afterEach(() => {
    runtime.search.mockReset();
    vi.useRealTimers();
  });

  it("does not render an in-flight response after the query is cleared", async () => {
    vi.useFakeTimers();
    let resolveSearch: ((value: unknown) => void) | undefined;
    runtime.search.mockReturnValue(
      new Promise((resolve) => {
        resolveSearch = resolve;
      }),
    );
    const element = document.createElement("div");
    const root = createRoot(element);

    act(() => root.render(<SearchScreen onOpenGame={() => undefined} />));
    const input = element.querySelector("input")!;
    const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    act(() => {
      setValue.call(input, "Deep Rock");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => {
      vi.advanceTimersByTime(300);
      await Promise.resolve();
    });
    expect(runtime.search).toHaveBeenCalledWith("Deep Rock", 30);

    act(() => {
      setValue.call(input, "");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => {
      resolveSearch?.({
        items: [{ app_id: 548430, name: "Deep Rock Galactic", release_state: "released" }],
        algorithm_version: "test",
      });
      await Promise.resolve();
    });

    expect(element.querySelector(".search-results")).toBeNull();
    expect(element.textContent).toContain("输入游戏名称开始搜索");
    act(() => root.unmount());
  });
});
