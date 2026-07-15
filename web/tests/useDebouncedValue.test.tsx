import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { useDebouncedValue } from "../src/app/useDebouncedValue";

// React 19 act() environment flag.
(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("useDebouncedValue", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("delays updates and collapses rapid changes to the last value", () => {
    vi.useFakeTimers();
    const seen = { current: "" };
    function Probe({ value }: { value: string }) {
      seen.current = useDebouncedValue(value, 300);
      return null;
    }
    const el = document.createElement("div");
    const root = createRoot(el);

    act(() => {
      root.render(<Probe value="a" />);
    });
    expect(seen.current).toBe("a");

    // Change; before the delay elapses it stays on the old value.
    act(() => {
      root.render(<Probe value="ab" />);
    });
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(seen.current).toBe("a");

    // Change again; the pending timer is cancelled, only the last value lands.
    act(() => {
      root.render(<Probe value="abc" />);
    });
    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(seen.current).toBe("abc");

    act(() => {
      root.unmount();
    });
  });
});
