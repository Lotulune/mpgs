// Primary nav: the four feed sections. Secondary nav: auxiliary entries
// (community / natural-language / search / calendar / settings). Both read
// the active state from the single `view` prop.

import { FEED_SECTIONS } from "../../api/types";
import { SECTION_META } from "../../app/format";
import { AUX_TABS, type ListView, type View } from "./nav";

export function NavTabs({
  view,
  onNavigate,
}: {
  view: View;
  onNavigate: (view: ListView) => void;
}) {
  const activeSection = view.kind === "feed" ? view.section : null;
  return (
    <>
      <nav className="tabs nav-primary" role="tablist" aria-label="主导航">
        {FEED_SECTIONS.map((s, idx) => (
          <button
            key={s}
            type="button"
            role="tab"
            className="tab"
            data-testid={`nav-feed-${s}`}
            aria-selected={activeSection === s}
            onClick={() => onNavigate({ kind: "feed", section: s })}
          >
            <span className="tab-key">{idx + 1}</span>
            {SECTION_META[s].label}
          </button>
        ))}
      </nav>
      <nav className="tabs nav-aux" role="tablist" aria-label="辅助入口">
        {AUX_TABS.map((tab) => (
          <button
            key={tab.view.kind}
            type="button"
            role="tab"
            className="tab"
            data-testid={`nav-${tab.view.kind}`}
            aria-label={tab.label}
            aria-selected={view.kind === tab.view.kind}
            title={tab.label}
            onClick={() => onNavigate(tab.view)}
          >
            <span className="tab-glyph" aria-hidden="true">
              {tab.glyph}
            </span>
            <span className="tab-label">{tab.label}</span>
          </button>
        ))}
      </nav>
    </>
  );
}
