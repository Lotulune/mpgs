import { describe, expect, it } from "vitest";
import type { CalendarItem } from "../src/api/types";
import {
  dayLabel,
  defaultWindow,
  groupByMonth,
  monthLabel,
  precisionLabel,
  toDayString,
} from "../src/app/calendar";

function item(appId: number, releaseDate: string | null): CalendarItem {
  return {
    app_id: appId,
    app_type: "game",
    canonical_name: `Game ${appId}`,
    release_state: releaseDate ? "coming_soon" : "unreleased",
    release_date: releaseDate,
    release_date_raw: releaseDate,
    release_date_precision: releaseDate ? "day" : "unknown",
    is_early_access: null,
    current_data_confidence: null,
    source_modified_at_ms: null,
    created_at_ms: 0,
    updated_at_ms: 0,
  };
}

describe("calendar helpers", () => {
  it("groups dated items by month in ascending order", () => {
    const groups = groupByMonth([
      item(1, "2026-09-15"),
      item(2, "2026-08-02"),
      item(3, "2026-08-20"),
    ]);
    expect(groups.map((g) => g.key)).toEqual(["2026-08", "2026-09"]);
    expect(groups[0]?.items.map((i) => i.app_id)).toEqual([2, 3]);
    expect(groups[1]?.items.map((i) => i.app_id)).toEqual([1]);
  });

  it("ignores items without a date in month grouping", () => {
    const groups = groupByMonth([item(1, null), item(2, "2026-08-02")]);
    expect(groups).toHaveLength(1);
    expect(groups[0]?.key).toBe("2026-08");
  });

  it("labels months and days", () => {
    expect(monthLabel("2026-08-02")).toBe("2026年 8 月");
    expect(monthLabel("bad")).toBeNull();
    expect(dayLabel("2026-08-02")).toBe("8月2日");
    expect(dayLabel(null)).toBe("日期未定");
  });

  it("labels precision and returns null when absent", () => {
    expect(precisionLabel("month")).toBe("预计月份");
    expect(precisionLabel(null)).toBeNull();
    expect(precisionLabel("weird")).toBe("weird");
  });

  it("builds a UTC day window clamped to one year", () => {
    const now = Date.UTC(2026, 6, 15); // 2026-07-15
    expect(toDayString(new Date(now))).toBe("2026-07-15");
    const w = defaultWindow(now, 6);
    expect(w.from).toBe("2026-07-15");
    expect(w.to).toBe("2027-01-15");
    const clamped = defaultWindow(now, 48);
    expect(clamped.to).toBe("2027-07-15"); // clamped to +12 months
  });
});
