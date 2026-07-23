// Compact theme picker for the topbar. Custom menu (not native <select>) so
// the whole control is one click target and the list can match theme tokens.

import { useEffect, useId, useRef, useState } from "react";
import { useTheme } from "../../app/ThemeProvider";
import { THEME_ORDER, THEMES } from "../../theme/registry";
import type { ThemeId } from "../../theme/types";

export function ThemeMenu() {
  const { themeId, setTheme } = useTheme();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const listId = useId();
  const current = THEMES[themeId];

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      setOpen(false);
      rootRef.current?.querySelector<HTMLButtonElement>(".theme-menu-trigger")?.focus();
    };
    window.addEventListener("mousedown", onPointerDown);
    // Capture on document so Escape is consumed before page-level shortcuts
    // such as the game-detail back handler reach window.
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown, true);
    };
  }, [open]);

  const pick = (id: ThemeId) => {
    setTheme(id);
    setOpen(false);
  };

  return (
    <div className="theme-menu" ref={rootRef}>
      <button
        type="button"
        className="theme-menu-trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        aria-label={`当前主题：${current.label}，点击更换`}
        onClick={() => setOpen((value) => !value)}
      >
        <span className="theme-menu-kicker">主题</span>
        <span className="theme-menu-name">{current.label}</span>
        <span className="theme-menu-chevron" aria-hidden="true">
          ▾
        </span>
      </button>
      {open && (
        <div
          id={listId}
          className="theme-menu-popover"
          role="listbox"
          aria-label="选择主题"
        >
          {THEME_ORDER.map((id) => {
            const selected = id === themeId;
            return (
              <button
                key={id}
                type="button"
                role="option"
                aria-selected={selected}
                className={selected ? "is-selected" : undefined}
                onClick={() => pick(id)}
              >
                <span className="theme-menu-option-label">{THEMES[id].label}</span>
                {selected && (
                  <span className="theme-menu-check" aria-hidden="true">
                    ✓
                  </span>
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
