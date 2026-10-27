// Text for one limit, shared by RingGauge and LimitBar. Only presentation: every number comes
// from the backend's `LimitView`; this picks the message and fills in its arguments.

import { t } from "../i18n/i18n.svelte";
import type { LimitView } from "../ipc/bindings";

export function isEstimate(limit: LimitView): boolean {
  return limit.source === "estimate";
}

/** The limit's display name: "5-hour window", "Weekly limit", "Fable · weekly". */
export function limitName(limit: LimitView): string {
  const { kind, scope } = limit;
  if (kind === "session") return t("limit-five-hour");
  if (kind === "weekly_scoped" && scope) {
    return t("limit-weekly-scoped", { name: scope.model ?? scope.surface ?? "" });
  }
  return t("limit-weekly");
}

/** "≈72 %" or "–". */
export function valueText(limit: LimitView | null): string {
  if (limit?.percent == null) return t("value-none");
  return t("value-pct", { pct: limit.percent, estimate: isEstimate(limit) ? "yes" : "no" });
}

/** The screen-reader sentence (design-language.md §7). `name` overrides `limitName`. */
export function ariaSentence(limit: LimitView | null, name: string, now: number): string {
  if (limit?.percent == null) return t("aria-limit-none", { name });
  const args = { name, pct: limit.percent, estimate: isEstimate(limit) ? "yes" : "no" };
  if (limit.resets_at == null) return t("aria-limit", args);
  return t("aria-limit-resets", { ...args, ms: Math.max(0, limit.resets_at - now) });
}

/** Percentage at reset for the ghost arc, when the backend projected one. */
export function projectedPercent(limit: LimitView | null): number | null {
  return limit?.projection?.projected_at_reset ?? null;
}

/** The arc length for `percent`, clamped to the ring. */
export function arc(percent: number | null): number {
  return percent == null ? 0 : Math.min(100, Math.max(0, percent));
}
