import { useMemo, useState } from "react";
import type {
  DemoFilter,
  LanguageFilter,
  LibraryFilters,
  ReleaseWindow,
} from "../types";

type FilterChipOption = { label: string; value: string };

const demoFilterOptions: Array<{ label: string; value: DemoFilter }> = [
  { value: "all", label: "全部" },
  { value: "demo_only", label: "仅 Demo" },
  { value: "released_with_demo", label: "Demo & 已发售" },
  { value: "released", label: "已发售" },
];

const releaseWindowOptions: Array<{ label: string; value: ReleaseWindow }> = [
  { value: "all", label: "不限" },
  { value: "week", label: "近一周" },
  { value: "month", label: "近一月" },
  { value: "quarter", label: "近三月" },
  { value: "year", label: "近一年" },
];

const languageFilterOptions: Array<{ label: string; value: LanguageFilter }> = [
  { value: "all", label: "全部语言" },
  { value: "schinese", label: "简体中文" },
  { value: "english", label: "英语" },
];

export function FilterPage({
  availableTags,
  defaultTagOptions,
  defaultFilters,
  filters,
  onApply,
  onCancel,
}: {
  availableTags: string[];
  defaultTagOptions: string[];
  defaultFilters: LibraryFilters;
  filters: LibraryFilters;
  onApply: (filters: LibraryFilters) => void;
  onCancel: () => void;
}) {
  const [draft, setDraft] = useState<LibraryFilters>(filters);
  const categoryOptions = useMemo<FilterChipOption[]>(
    () =>
      Array.isArray(availableTags)
        ? [...new Set([...defaultTagOptions, ...availableTags])]
            .slice(0, 8)
            .map((tag) => ({ label: tag, value: tag }))
        : defaultTagOptions.slice(0, 8).map((tag) => ({ label: tag, value: tag })),
    [availableTags, defaultTagOptions],
  );
  const tagOptions = useMemo<FilterChipOption[]>(
    () =>
      Array.isArray(availableTags)
        ? availableTags.map((tag) => ({ label: tag, value: tag }))
        : [],
    [availableTags],
  );

  function toggleDraftTag(tag: string) {
    setDraft((current) => {
      const selectedTags = Array.isArray(current.selectedTags)
        ? current.selectedTags
        : [];
      return {
        ...current,
        selectedTags: selectedTags.includes(tag)
          ? selectedTags.filter((item) => item !== tag)
          : [...selectedTags, tag],
      };
    });
  }

  return (
    <section className="filter-page">
      <div className="detail-toolbar">
        <button type="button" onClick={onCancel}>
          ←
        </button>
        <h2>筛选器</h2>
        <button type="button" onClick={() => setDraft({ ...defaultFilters })}>
          ↻ 重置
        </button>
      </div>

      <FilterChipGroup
        allowMultiple={false}
        chips={demoFilterOptions}
        selectedValues={[draft.demoFilter]}
        title="Demo 状态"
        onToggle={(value) =>
          setDraft((current) => ({ ...current, demoFilter: value as DemoFilter }))
        }
      />
      <FilterChipGroup
        allowMultiple={false}
        chips={releaseWindowOptions}
        selectedValues={[draft.releaseWindow]}
        title="发售时间"
        onToggle={(value) =>
          setDraft((current) => ({
            ...current,
            releaseWindow: value as ReleaseWindow,
          }))
        }
      />
      <FilterChipGroup
        allowMultiple={false}
        chips={languageFilterOptions}
        selectedValues={[draft.selectedLanguage]}
        title="语言支持"
        onToggle={(value) =>
          setDraft((current) => ({
            ...current,
            selectedLanguage: value as LanguageFilter,
          }))
        }
      />
      <FilterChipGroup
        allowMultiple
        chips={categoryOptions}
        selectedValues={draft.selectedTags}
        title="游戏分类"
        onToggle={toggleDraftTag}
      />

      <label className="wide-range">
        <span>
          Steam 好评度
          <b className="range-current">{draft.minReviewPct}%</b>
        </span>
        <div className="range-control-row">
          <input
            max="100"
            min="0"
            value={draft.minReviewPct}
            onChange={(event) => {
              const value = Number(event.currentTarget.value);
              setDraft((current) => ({ ...current, minReviewPct: value }));
            }}
            type="range"
          />
          <input
            className="range-number-input"
            max="100"
            min="0"
            type="number"
            value={draft.minReviewPct}
            onChange={(event) => {
              const value = Number(event.currentTarget.value);
              setDraft((current) => ({
                ...current,
                minReviewPct: Number.isFinite(value)
                  ? Math.max(0, Math.min(100, value))
                  : 0,
              }));
            }}
          />
        </div>
        <span>
          <b>0%</b>
          <b>100%</b>
        </span>
      </label>

      <label className="wide-range">
        <span>
          当前在线人数下限
          <b className="range-current">{draft.minPlayers}</b>
        </span>
        <div className="range-control-row">
          <input
            max="1000"
            min="0"
            value={draft.minPlayers}
            onChange={(event) => {
              const value = Number(event.currentTarget.value);
              setDraft((current) => ({ ...current, minPlayers: value }));
            }}
            type="range"
          />
          <input
            className="range-number-input"
            max="1000"
            min="0"
            type="number"
            value={draft.minPlayers}
            onChange={(event) => {
              const value = Number(event.currentTarget.value);
              setDraft((current) => ({
                ...current,
                minPlayers: Number.isFinite(value)
                  ? Math.max(0, Math.min(1000, value))
                  : 0,
              }));
            }}
          />
        </div>
        <span>
          <b>0</b>
          <b>1000+</b>
        </span>
      </label>

      <FilterChipGroup
        allowMultiple
        chips={tagOptions}
        selectedValues={draft.selectedTags}
        title="游戏标签（可多选）"
        onToggle={toggleDraftTag}
      />

      <button
        aria-pressed={draft.hideAdultContent}
        className={draft.hideAdultContent ? "toggle-row active" : "toggle-row"}
        onClick={() =>
          setDraft((current) => ({
            ...current,
            hideAdultContent: !current.hideAdultContent,
          }))
        }
        type="button"
      >
        <span>隐藏成人内容</span>
        <i />
        <small>{draft.hideAdultContent ? "已隐藏" : "未隐藏"}</small>
      </button>

      <div className="filter-actions">
        <button type="button" onClick={onCancel}>
          取消
        </button>
        <button className="gold-button" type="button" onClick={() => onApply(draft)}>
          应用筛选
        </button>
      </div>
    </section>
  );
}

function FilterChipGroup({
  allowMultiple,
  chips,
  selectedValues,
  title,
  onToggle,
}: {
  allowMultiple: boolean;
  chips: FilterChipOption[];
  selectedValues: string[];
  title: string;
  onToggle: (value: string) => void;
}) {
  return (
    <div className="filter-group">
      <h3>{title}</h3>
      <div>
        {chips.map((chip) => {
          const isActive =
            Array.isArray(selectedValues) && selectedValues.includes(chip.value);
          return (
            <button
              aria-pressed={isActive}
              className={isActive ? "active" : ""}
              key={chip.value}
              onClick={() => onToggle(chip.value)}
              type="button"
            >
              {chip.label}
              {allowMultiple && isActive ? " ×" : ""}
            </button>
          );
        })}
      </div>
    </div>
  );
}
