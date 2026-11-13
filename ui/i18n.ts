import { locale } from "nsh";
import { availableLocales, setLocale, t } from "nsh/i18n";

export type AppLocale = "zh-CN" | "en";

export const DEFAULT_LOCALE: AppLocale = "zh-CN";

export const SUPPORTED_LOCALES: readonly { id: AppLocale; labelKey: string; nativeLabel: string }[] = [
  { id: "zh-CN", labelKey: "common.locale_zh", nativeLabel: "简体中文" },
  { id: "en", labelKey: "common.locale_en", nativeLabel: "English" },
];

/**
 * Normalize any BCP-47 / system locale tag to a supported app locale.
 */
export function normalizeLocale(raw?: string | null): AppLocale {
  const code = (raw ?? "").trim().toLowerCase();
  if (code === "en" || code.startsWith("en-") || code.startsWith("en_")) {
    return "en";
  }
  return "zh-CN";
}

/**
 * Read the reactive current app locale. Calling inside JSX/computed views subscribes to locale changes.
 */
export function getLocale(): AppLocale {
  return normalizeLocale(locale());
}

/**
 * Switch the active NSH runtime locale and return the normalized locale applied.
 */
export function applyLocale(next: AppLocale): AppLocale {
  const normalized = normalizeLocale(next);
  setLocale(normalized);
  return normalized;
}

/**
 * Toggle cleanly between Simplified Chinese and English.
 */
export function toggleLocale(): AppLocale {
  const current = getLocale();
  return applyLocale(current === "zh-CN" ? "en" : "zh-CN");
}

/**
 * List locales reported by the runtime, guaranteed to include supported app locales.
 */
export function listAvailableLocales(): string[] {
  const found = availableLocales();
  return found.length > 0 ? found : ["zh-CN", "en"];
}

/**
 * Convenience check for branch logic such as bilingual idioms or date word order.
 */
export function isEnglishLocale(loc?: AppLocale): boolean {
  return (loc ?? getLocale()) === "en";
}

/**
 * Translate a dot-separated key using rust-i18n `%{var}` interpolation.
 * Subscribes reactively to `nsh/i18n` locale updates.
 */
export function tr(key: string, params?: Record<string, string | number>): string {
  if (!params) {
    return t(key);
  }
  return t(key, params);
}

/**
 * Format a numeric value according to the active app locale, with safe JS-engine fallback.
 */
export function formatNumber(value: number, fractionDigits = 0): string {
  if (!Number.isFinite(value)) {
    return "--";
  }
  const active = getLocale();
  try {
    if (typeof Intl !== "undefined" && typeof Intl.NumberFormat === "function") {
      return new Intl.NumberFormat(active, {
        minimumFractionDigits: fractionDigits,
        maximumFractionDigits: fractionDigits,
      }).format(value);
    }
  } catch {
    // Fall through to deterministic formatting when Intl ICU data is absent.
  }
  return fractionDigits > 0 ? value.toFixed(fractionDigits) : String(Math.round(value));
}

/**
 * Format an integer or percentage readout cleanly in Chinese or English.
 */
export function formatPercent(value: number, fractionDigits = 0): string {
  if (!Number.isFinite(value)) {
    return "--";
  }
  return `${formatNumber(value, fractionDigits)}%`;
}

/**
 * Format an hour duration readout (e.g., "7.4 小时" vs "7.4 h").
 */
export function formatDurationHours(hours: number, fractionDigits = 1): string {
  if (!Number.isFinite(hours)) {
    return "--";
  }
  return tr("common.hours_unit", { value: formatNumber(hours, fractionDigits) });
}

const EN_SHORT_MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/**
 * Format an ISO YYYY-MM-DD date or timestamp as a compact month-day string.
 */
export function formatMonthDay(isoOrDate: string | Date): string {
  const date = isoOrDate instanceof Date ? isoOrDate : parseIsoDateSafe(isoOrDate);
  if (!date) {
    return typeof isoOrDate === "string" ? isoOrDate : "";
  }
  const month = date.getMonth() + 1;
  const day = date.getDate();
  if (isEnglishLocale()) {
    return `${EN_SHORT_MONTHS[month - 1] ?? String(month)} ${day}`;
  }
  return `${month}月${day}日`;
}

/**
 * Format an ISO YYYY-MM-DD date or timestamp with year in the active locale.
 */
export function formatShortDate(isoOrDate: string | Date): string {
  const date = isoOrDate instanceof Date ? isoOrDate : parseIsoDateSafe(isoOrDate);
  if (!date) {
    return typeof isoOrDate === "string" ? isoOrDate : "";
  }
  const year = date.getFullYear();
  const month = date.getMonth() + 1;
  const day = date.getDate();
  if (isEnglishLocale()) {
    return `${EN_SHORT_MONTHS[month - 1] ?? String(month)} ${day}, ${year}`;
  }
  return `${year}年${month}月${day}日`;
}

function parseIsoDateSafe(raw: string): Date | null {
  const trimmed = raw.trim();
  if (!trimmed) {
    return null;
  }
  const ymd = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(trimmed);
  if (ymd) {
    const year = Number(ymd[1]);
    const month = Number(ymd[2]) - 1;
    const day = Number(ymd[3]);
    const parsed = new Date(year, month, day);
    return Number.isNaN(parsed.getTime()) ? null : parsed;
  }
  const parsed = new Date(trimmed);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}
