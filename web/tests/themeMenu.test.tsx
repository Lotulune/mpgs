import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeProvider } from "../src/app/ThemeProvider";
import { ThemeMenu } from "../src/screens/shell/ThemeMenu";
import { MemoryStorage } from "./helpers";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ThemeMenu", () => {
  let host: HTMLDivElement;

  beforeEach(() => {
    host = document.createElement("div");
    document.body.append(host);
    // Keep theme persistence out of real localStorage.
    vi.stubGlobal("localStorage", new MemoryStorage());
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
  });

  afterEach(() => {
    host.remove();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("shows the current theme on a single full-hit trigger and lists options", () => {
    const root = createRoot(host);
    act(() => {
      root.render(
        <ThemeProvider initialTheme="wafu">
          <ThemeMenu />
        </ThemeProvider>,
      );
    });

    const trigger = host.querySelector<HTMLButtonElement>(".theme-menu-trigger");
    expect(trigger).toBeTruthy();
    expect(trigger?.textContent).toContain("主题");
    expect(trigger?.textContent).toContain("樱树和风");
    expect(host.querySelector(".theme-menu-popover")).toBeNull();

    act(() => {
      trigger?.click();
    });

    const options = host.querySelectorAll<HTMLButtonElement>(".theme-menu-popover [role='option']");
    expect(options.length).toBeGreaterThanOrEqual(5);
    const selected = host.querySelector(".theme-menu-popover [aria-selected='true']");
    expect(selected?.textContent).toContain("樱树和风");

    const steam = Array.from(options).find((el) => el.textContent?.includes("Steam"));
    expect(steam).toBeTruthy();
    act(() => {
      steam?.click();
    });

    expect(host.querySelector(".theme-menu-popover")).toBeNull();
    expect(host.querySelector(".theme-menu-trigger")?.textContent).toContain("Steam");

    act(() => root.unmount());
  });

  it("consumes Escape before page-level window shortcuts", () => {
    const underlyingShortcut = vi.fn();
    window.addEventListener("keydown", underlyingShortcut);
    const root = createRoot(host);
    try {
      act(() => {
        root.render(
          <ThemeProvider initialTheme="minimal">
            <ThemeMenu />
          </ThemeProvider>,
        );
      });

      const trigger = host.querySelector<HTMLButtonElement>(".theme-menu-trigger")!;
      act(() => trigger.click());
      expect(host.querySelector(".theme-menu-popover")).not.toBeNull();

      act(() => {
        trigger.dispatchEvent(
          new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
        );
      });

      expect(host.querySelector(".theme-menu-popover")).toBeNull();
      expect(underlyingShortcut).not.toHaveBeenCalled();
      expect(document.activeElement).toBe(trigger);
    } finally {
      window.removeEventListener("keydown", underlyingShortcut);
      act(() => root.unmount());
    }
  });
});
