// The overview's layout steps (Step 15b). Values, shares, the week-over-week change and the plan
// ratio come from the backend (`get_overview`); these functions only pick what to show and size
// bars and sparklines from it.

import type { ChartSpan } from "../../../lib/chart/model";
import type {
  Basis,
  CalibrationView,
  ModelSlice,
  Overview,
  TimeSeries,
  TokenBreakdown,
} from "../../../lib/ipc/bindings";
import { usd } from "../../panel/detail";

export { usd };

/** One bar of a magnitude list: `width` is 0–100 of the largest row, `share` 0–100 of all. */
export interface BarRow<K extends string = string> {
  key: K;
  value: number;
  share: number;
  width: number;
}

function bars<K extends string>(rows: readonly { key: K; value: number }[]): BarRow<K>[] {
  const total = rows.reduce((sum, r) => sum + r.value, 0);
  const max = Math.max(0, ...rows.map((r) => r.value));
  return rows.map((r) => ({
    ...r,
    share: total > 0 ? (r.value / total) * 100 : 0,
    width: max > 0 ? (r.value / max) * 100 : 0,
  }));
}

export type TokenKind = "input" | "output" | "cache_read" | "cache_write";

/**
 * Token kinds as bars, in a fixed order (never re-sorted, so a kind keeps its row). Cache reads
 * usually dwarf the rest, which is why these are rows and not one stacked bar: a stacked bar
 * would draw input and output as slivers.
 */
export function tokenRows(t: TokenBreakdown): BarRow<TokenKind>[] {
  return bars<TokenKind>([
    { key: "input", value: t.tokens.input },
    { key: "output", value: t.tokens.output },
    { key: "cache_read", value: t.tokens.cache_read },
    { key: "cache_write", value: t.tokens.cache_write },
  ]);
}

/** Models by cost, most expensive first (the backend's order); at most `max`, the rest folded. */
export function modelRows(models: readonly ModelSlice[], max = 5): BarRow[] {
  const shown = models.slice(0, max).map((m) => ({ key: m.model, value: m.cost }));
  const rest = models.slice(max).reduce((sum, m) => sum + m.cost, 0);
  if (rest > 0) shown.push({ key: "", value: rest });
  return bars(shown);
}

/**
 * The session spans under the 7-day chart: the weekly limit's markers (the 5-hour window's
 * when there is no weekly series), the `max` largest by share, then by cost order.
 */
export function spansOf(series: readonly TimeSeries[], max = 5): ChartSpan[] {
  const source =
    series.find((s) => s.limit.kind === "weekly_all") ??
    series.find((s) => s.limit.kind === "session");
  if (!source) return [];
  return source.markers
    .map((m, rank) => ({ m, rank }))
    .sort((a, b) => (b.m.percent ?? -1) - (a.m.percent ?? -1) || a.rank - b.rank)
    .slice(0, max)
    .map(({ m }) => ({
      id: String(m.session_id),
      from: m.start,
      to: m.end,
      label: m.name.text,
      percent: m.percent,
    }));
}

/** A `seven_day_breakdown` key's label key; unknown keys are shown as they come. */
export function surfaceKey(key: string): string | null {
  switch (key) {
    case "claude_code":
      return "overview-surface-claude-code";
    case "chat":
      return "overview-surface-chat";
    case "cowork":
      return "overview-surface-cowork";
    case "other":
      return "overview-surface-other";
    default:
      return null;
  }
}

/** Whether the week's shares are settled: split with a calibration that is at least medium. */
export function sharesSettled(basis: Basis | null): boolean {
  return basis != null && basis.method === "calibrated" && !basis.dim;
}

/**
 * What the page says about its data, besides the sections themselves:
 * - `no-limits`: no limit reading at all (no snapshot yet);
 * - `estimates-only`: every limit shown is our own estimate;
 * - `learning`: shares are not split with a usable calibration yet.
 */
export interface DataState {
  noLimits: boolean;
  estimatesOnly: boolean;
  learning: CalibrationView | "none" | null;
}

export function dataState(o: Overview): DataState {
  const measured = o.limits.filter((l) => l.source !== "estimate");
  return {
    noLimits: o.limits.length === 0,
    estimatesOnly: o.limits.length > 0 && measured.length === 0,
    learning: sharesSettled(o.basis) ? null : (o.calibration ?? "none"),
  };
}

/** The change as a direction and whole percent; under half a percent is "flat". */
export function changeOf(
  change: number | null,
): { dir: "up" | "down" | "flat"; pct: number } | null {
  if (change == null || !Number.isFinite(change)) return null;
  const pct = Math.abs(change) * 100;
  if (pct < 0.5) return { dir: "flat", pct: 0 };
  return { dir: change > 0 ? "up" : "down", pct };
}

/** A count shortened for `overview-count`: `{ unit: "m", n: 3.1 }` is 3.1 million. */
export function countOf(n: number): { unit: "one" | "k" | "m" | "b"; n: number } {
  const abs = Math.abs(n);
  if (abs >= 1e9) return { unit: "b", n: n / 1e9 };
  if (abs >= 1e6) return { unit: "m", n: n / 1e6 };
  if (abs >= 1e4) return { unit: "k", n: n / 1e3 };
  return { unit: "one", n };
}
