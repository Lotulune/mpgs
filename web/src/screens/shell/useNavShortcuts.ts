// Keyboard: 1-4 switch feed sections; "/" opens search. Ignored while typing.

import { useEffect } from "react";
import { FEED_SECTIONS } from "../../api/types";
import type { ListView } from "./nav";

export function useNavShortcuts(onNavigate: (view: ListView) => void): void {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
      if (event.key === "/") {
        event.preventDefault();
        onNavigate({ kind: "search" });
        return;
      }
      const idx = Number(event.key) - 1;
      const next = FEED_SECTIONS[idx];
      if (idx >= 0 && idx < FEED_SECTIONS.length && next) {
        onNavigate({ kind: "feed", section: next });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onNavigate]);
}
