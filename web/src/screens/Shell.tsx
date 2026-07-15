// App shell: brand, primary nav (4 feed sections + search/calendar/settings),
// theme switcher, FX intensity, connectivity status. Hosts all view screens.

import { useCallback, useEffect, useRef, useState } from "react";
import type { FeedSection } from "../api/types";
import { FEED_SECTIONS } from "../api/types";
import { feedbackQueue } from "../app/runtime";
import { useTheme } from "../app/ThemeProvider";
import { SECTION_META } from "../app/format";
import { THEME_ORDER, THEMES } from "../theme/registry";
import type { FxIntensity } from "../fx/types";
import { FeedScreen } from "./FeedScreen";
import { GameDetailScreen } from "./GameDetailScreen";
import { SearchScreen } from "./SearchScreen";
import { CalendarScreen } from "./CalendarScreen";
import { SettingsScreen } from "./SettingsScreen";

type ListView =
  | { kind: "feed"; section: FeedSection }
  | { kind: "search" }
  | { kind: "calendar" }
  | { kind: "settings" };

type View = ListView | { kind: "game"; appId: number };

const FX_LABELS: Record<FxIntensity, string> = { off: "特效关", low: "特效低", full: "特效全" };
const FX_CYCLE: FxIntensity[] = ["full", "low", "off"];

const AUX_TABS: { view: ListView; label: string; glyph: string }[] = [
  { view: { kind: "search" }, label: "搜索", glyph: "⌕" },
  { view: { kind: "calendar" }, label: "日历", glyph: "▦" },
  { view: { kind: "settings" }, label: "设置", glyph: "⚙" },
];

export function Shell() {
  const { themeId, setTheme, intensity, setIntensity } = useTheme();
  const [view, setView] = useState<View>({ kind: "feed", section: "recent_release" });
  const [online, setOnline] = useState(() => navigator.onLine);
  const [pendingCount, setPendingCount] = useState(() => feedbackQueue.pendingCount());
  // Where the game detail returns to (the list the user opened it from).
  const lastListView = useRef<ListView>({ kind: "feed", section: "recent_release" });
  useEffect(() => {
    if (view.kind !== "game") lastListView.current = view;
  }, [view]);

  useEffect(() => {
    const update = () => setOnline(navigator.onLine);
    window.addEventListener("online", update);
    window.addEventListener("offline", update);
    return () => {
      window.removeEventListener("online", update);
      window.removeEventListener("offline", update);
    };
  }, []);

  useEffect(() => {
    return feedbackQueue.subscribe(() => setPendingCount(feedbackQueue.pendingCount()));
  }, []);

  // Keyboard: 1-4 switch feed sections; "/" opens search. Ignored while typing.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
      if (event.key === "/") {
        event.preventDefault();
        setView({ kind: "search" });
        return;
      }
      const idx = Number(event.key) - 1;
      const next = FEED_SECTIONS[idx];
      if (idx >= 0 && idx < FEED_SECTIONS.length && next) {
        setView({ kind: "feed", section: next });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const openGame = useCallback((appId: number) => setView({ kind: "game", appId }), []);
  const backToList = useCallback(() => setView(lastListView.current), []);

  const cycleFx = () => {
    const idx = FX_CYCLE.indexOf(intensity);
    const next = FX_CYCLE[(idx + 1) % FX_CYCLE.length] ?? "full";
    setIntensity(next);
  };

  const activeSection = view.kind === "feed" ? view.section : null;

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">
          MPGS
          <small>熟人联机推荐</small>
        </div>
        <nav className="tabs" role="tablist" aria-label="主导航">
          {FEED_SECTIONS.map((s, idx) => (
            <button
              key={s}
              type="button"
              role="tab"
              className="tab"
              aria-selected={activeSection === s}
              onClick={() => setView({ kind: "feed", section: s })}
            >
              <span className="tab-key">{idx + 1}</span>
              {SECTION_META[s].label}
            </button>
          ))}
          <span className="tab-sep" aria-hidden="true" />
          {AUX_TABS.map((tab) => (
            <button
              key={tab.view.kind}
              type="button"
              role="tab"
              className="tab"
              aria-selected={view.kind === tab.view.kind}
              onClick={() => setView(tab.view)}
            >
              <span className="tab-glyph" aria-hidden="true">
                {tab.glyph}
              </span>
              {tab.label}
            </button>
          ))}
        </nav>
        <div className="topbar-controls">
          {!online && <span className="chip danger">离线</span>}
          {pendingCount > 0 && <span className="chip warn">{pendingCount} 条待同步</span>}
          <label className="sr-label">
            <span className="sr-only">切换主题</span>
            <select
              className="btn small"
              value={themeId}
              onChange={(event) => {
                const next = event.target.value;
                if (next in THEMES) setTheme(next as keyof typeof THEMES);
              }}
              aria-label="切换主题"
            >
              {THEME_ORDER.map((id) => (
                <option key={id} value={id}>
                  {THEMES[id].label}
                </option>
              ))}
            </select>
          </label>
          <button type="button" className="btn small ghost" onClick={cycleFx} aria-label="切换特效强度">
            {FX_LABELS[intensity]}
          </button>
        </div>
      </header>

      <main className="main">
        {view.kind === "feed" && <FeedScreen section={view.section} onOpenGame={openGame} />}
        {view.kind === "search" && <SearchScreen onOpenGame={openGame} />}
        {view.kind === "calendar" && <CalendarScreen onOpenGame={openGame} />}
        {view.kind === "settings" && <SettingsScreen />}
        {view.kind === "game" && <GameDetailScreen appId={view.appId} onBack={backToList} />}
      </main>
    </div>
  );
}
