// Theme + FX intensity context, and the canvas host that feeds the engine.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { fxEngine, preferredIntensity } from "../fx/engine";
import type { ActionKind, FxIntensity } from "../fx/types";
import { activateTheme, loadSavedTheme, saveTheme, THEMES } from "../theme/registry";
import type { ThemeDefinition, ThemeId } from "../theme/types";
import { loadFxIntensity, saveFxIntensity } from "./runtime";

interface ThemeContextValue {
  themeId: ThemeId;
  theme: ThemeDefinition;
  setTheme: (id: ThemeId) => void;
  intensity: FxIntensity;
  setIntensity: (value: FxIntensity) => void;
  /** Fire a semantic action effect centered on an element. */
  fireAction: (kind: ActionKind, element: Element | null) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function useTheme(): ThemeContextValue {
  const value = useContext(ThemeContext);
  if (!value) throw new Error("useTheme outside ThemeProvider");
  return value;
}

function initialIntensity(): FxIntensity {
  const saved = loadFxIntensity();
  if (saved === "off" || saved === "low" || saved === "full") return saved;
  return preferredIntensity();
}

export function ThemeProvider({
  initialTheme,
  children,
}: {
  initialTheme?: ThemeId;
  children: ReactNode;
}) {
  const [themeId, setThemeId] = useState<ThemeId>(
    () => initialTheme ?? loadSavedTheme() ?? "steam",
  );
  const [intensity, setIntensityState] = useState<FxIntensity>(initialIntensity);
  const ambientRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const ambient = ambientRef.current;
    const overlay = overlayRef.current;
    if (!ambient || !overlay) return;
    fxEngine.attach(ambient, overlay);
    return () => fxEngine.detach();
  }, []);

  useEffect(() => {
    activateTheme(themeId);
  }, [themeId]);

  useEffect(() => {
    fxEngine.setIntensity(intensity);
  }, [intensity]);

  // Global pointer-down click feedback. Skips scrollbar-ish edge clicks.
  useEffect(() => {
    const onPointerDown = (event: PointerEvent) => {
      if (event.button !== 0) return;
      fxEngine.click(event.clientX, event.clientY);
    };
    window.addEventListener("pointerdown", onPointerDown, { passive: true });
    return () => window.removeEventListener("pointerdown", onPointerDown);
  }, []);

  const setTheme = useCallback((id: ThemeId) => {
    setThemeId(id);
    saveTheme(id);
  }, []);

  const setIntensity = useCallback((value: FxIntensity) => {
    setIntensityState(value);
    saveFxIntensity(value);
  }, []);

  const fireAction = useCallback((kind: ActionKind, element: Element | null) => {
    if (element) {
      fxEngine.actionAt(kind, element);
    }
  }, []);

  const value = useMemo<ThemeContextValue>(
    () => ({
      themeId,
      theme: THEMES[themeId],
      setTheme,
      intensity,
      setIntensity,
      fireAction,
    }),
    [themeId, setTheme, intensity, setIntensity, fireAction],
  );

  return (
    <ThemeContext.Provider value={value}>
      <canvas ref={ambientRef} className="fx-ambient" aria-hidden="true" />
      {children}
      <canvas ref={overlayRef} className="fx-overlay" aria-hidden="true" />
    </ThemeContext.Provider>
  );
}
