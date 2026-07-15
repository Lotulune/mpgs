// Presentation helpers. Unknown data must render as 未知, never as a guess
// (PRD 体验原则: 数据未知时显示未知).

import type { FeedSection } from "../api/types";

export const SECTION_META: Record<FeedSection, { label: string; hint: string }> = {
  recent_release: { label: "最近发售", hint: "180 天内的新作" },
  upcoming: { label: "即将发售 / Demo", hint: "Coming Soon 与试玩" },
  popular_legacy: { label: "人气老游", hint: "仍然活跃的口碑之作" },
  classic_legacy: { label: "经典老游", hint: "自建服与私联经典" },
};

const DOMINANT_MODE_LABELS: Record<string, string> = {
  private_coop: "私人合作",
  coop: "合作",
  online_coop: "在线合作",
  self_hosted: "自建服务器",
  dedicated_server: "专用服务器",
  p2p: "P2P 联机",
  matchmaking: "公共匹配",
  matchmaking_core: "公共匹配核心",
  mmo: "MMO",
  pvp: "对抗",
  mixed: "混合模式",
};

export function dominantModeLabel(mode: string | null): string {
  if (!mode) return "未知";
  return DOMINANT_MODE_LABELS[mode] ?? mode;
}

export function partyLabel(min: number | null, max: number | null): string {
  if (min === null && max === null) return "人数未知";
  if (min !== null && max !== null) {
    return min === max ? `${min} 人` : `${min}–${max} 人`;
  }
  if (max !== null) return `最多 ${max} 人`;
  return `至少 ${min} 人`;
}

export function formatPrice(minor: number | null, currency: string | null, isFree: boolean | null): string {
  if (isFree === true) return "免费";
  if (minor === null || !currency) return "价格未知";
  const major = minor / 100;
  const symbol = currency === "CNY" ? "¥" : currency === "USD" ? "$" : `${currency} `;
  return `${symbol}${major % 1 === 0 ? major.toFixed(0) : major.toFixed(2)}`;
}

export function formatPercent(value: number | null): string {
  if (value === null || Number.isNaN(value)) return "未知";
  return `${Math.round(value * 100)}%`;
}

export function formatCount(value: number | null): string {
  if (value === null) return "未知";
  if (value >= 10_000) return `${(value / 10_000).toFixed(1)} 万`;
  return String(value);
}

export function positiveRate(total: number | null, positive: number | null): string {
  if (!total || positive === null) return "未知";
  return `${Math.round((positive / total) * 100)}% 好评`;
}

export function formatAgo(ms: number, now: number = Date.now()): string {
  const delta = Math.max(0, now - ms);
  const minutes = Math.floor(delta / 60_000);
  if (minutes < 1) return "刚刚";
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} 天前`;
  const months = Math.floor(days / 30);
  return `${months} 个月前`;
}

/** Data older than this renders a staleness warning (NFR-004). */
export const STALE_AFTER_MS = 24 * 60 * 60 * 1000;

export function isStale(dataUpdatedAtMs: number, now: number = Date.now()): boolean {
  return now - dataUpdatedAtMs > STALE_AFTER_MS;
}

const RELEASE_STATE_LABELS: Record<string, string> = {
  released: "已发售",
  coming_soon: "即将发售",
  early_access: "抢先体验",
  unreleased: "未发售",
  demo: "试玩",
  playtest: "Playtest",
  delisted: "已下架",
};

export function releaseStateLabel(state: string): string {
  return RELEASE_STATE_LABELS[state] ?? state;
}

const PLATFORM_LABELS: Record<string, string> = {
  windows: "Windows",
  mac: "macOS",
  macos: "macOS",
  linux: "Linux",
  steamdeck: "Steam Deck",
};

export function platformLabels(platforms: string[]): string {
  if (platforms.length === 0) return "未知";
  return platforms.map((p) => PLATFORM_LABELS[p] ?? p).join(" / ");
}

const LANGUAGE_LABELS: Record<string, string> = {
  schinese: "简体中文",
  tchinese: "繁体中文",
  english: "英语",
  japanese: "日语",
  koreana: "韩语",
  russian: "俄语",
  german: "德语",
  french: "法语",
  spanish: "西班牙语",
  latam: "拉美西语",
};

export function languageLabels(languages: string[]): string {
  if (languages.length === 0) return "未知";
  return languages.map((l) => LANGUAGE_LABELS[l] ?? l).join("、");
}

export const FEEDBACK_LABELS: Record<string, string> = {
  like: "喜欢",
  not_interested: "不感兴趣",
  played: "玩过",
  too_competitive: "太竞技",
  party_size_mismatch: "人数不合适",
  hosting_friction: "开服麻烦",
};

const FEATURE_LABELS: Record<string, string> = {
  private_session: "私人房间",
  online_coop: "在线合作",
  self_hosted_server: "自建服务器",
  multiplayer_category: "多人分类",
  dedicated_server: "专用服务器",
  drop_in_out: "随进随出",
};

export function featureLabel(feature: string): string {
  return FEATURE_LABELS[feature] ?? feature;
}

const SOURCE_TYPE_LABELS: Record<string, string> = {
  official_api: "官方接口",
  official_store: "官方商店",
  store_adapter: "商店适配器",
  manual: "人工确认",
  inference: "推断",
  ai_inference: "AI 推断",
};

export function sourceTypeLabel(type: string): string {
  return SOURCE_TYPE_LABELS[type] ?? type;
}

export function evidenceValueLabel(value: unknown): string {
  if (value === true) return "是";
  if (value === false) return "否";
  if (value === null || value === undefined) return "未知";
  if (typeof value === "number" || typeof value === "string") return String(value);
  return JSON.stringify(value);
}
