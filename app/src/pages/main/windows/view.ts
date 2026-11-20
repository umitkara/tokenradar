// Layout for the windows and patterns pages (Step 17a). The backend sends finished values
// (days, shares, cells, weeks); these functions only place them: bar heights, colour steps,
// stacked-area paths and the order of the week's days.

import type { CycleRow, HeatCell, MixTrend, ModelFamily } from "../../../lib/ipc/bindings";

/** The largest day value of the rows: percent when the days have one, cost otherwise. */
export function dayMax(rows: readonly CycleRow[]): { max: number; byCost: boolean } {
  const byCost = !rows.some((r) => r.days.some((d) => d.percent != null));
  let max = 0;
  for (const r of rows) {
    for (const d of r.days) max = Math.max(max, byCost ? d.cost : (d.percent ?? 0));
  }
  return { max, byCost };
}

/** A bar's height in percent of its track; 0 for an empty day. */
export function barHeight(value: number, max: number): number {
  return max > 0 && value > 0 ? Math.min(100, (value / max) * 100) : 0;
}

/**
 * The locale's first day of the week, Monday = 0. `Intl.Locale#getWeekInfo` when the browser
 * knows it (and the UI language matches the system's), otherwise Monday for Turkish and
 * Sunday for English.
 */
export function firstWeekday(ui: "en" | "tr", system: string | undefined): number {
  const tag =
    system && system.slice(0, 2).toLowerCase() === ui ? system : ui === "tr" ? "tr-TR" : "en-US";
  try {
    const loc = new Intl.Locale(tag) as Intl.Locale & {
      getWeekInfo?: () => { firstDay: number };
      weekInfo?: { firstDay: number };
    };
    const info = loc.getWeekInfo?.() ?? loc.weekInfo;
    if (info && info.firstDay >= 1 && info.firstDay <= 7) return info.firstDay - 1;
  } catch {
    // An unknown tag: fall through to the table.
  }
  return ui === "tr" ? 0 : 6;
}

/** Monday-zero week days in display order. */
export function weekOrder(first: number): number[] {
  return Array.from({ length: 7 }, (_, i) => (first + i) % 7);
}

/** Monday-zero → `Date#getDay` (Sunday = 0), for `weekdayName`. */
export function toJsDay(mondayZero: number): number {
  return (mondayZero + 1) % 7;
}

/** Colour steps of the heat map, empty excluded. */
export const HEAT_STEPS = 5;

/** A cell's step, 0 (no usage) to `HEAT_STEPS`; any usage is at least step 1. */
export function heatStep(total: number, max: number): number {
  if (total <= 0 || max <= 0) return 0;
  return Math.max(1, Math.min(HEAT_STEPS, Math.ceil((total / max) * HEAT_STEPS)));
}

export function heatCell(
  cells: readonly HeatCell[],
  weekday: number,
  hour: number,
): HeatCell | null {
  return cells[weekday * 24 + hour] ?? null;
}

/**
 * Stacked areas, the first series at the bottom: one closed SVG path per series in a
 * `width × height` box, `yMax` at the top. Points are evenly spaced, one per value.
 */
export function stackPaths(
  series: readonly (readonly number[])[],
  width: number,
  height: number,
  yMax: number,
): string[] {
  const n = series[0]?.length ?? 0;
  if (n === 0 || yMax <= 0) return series.map(() => "");
  const x = (i: number) => (n === 1 ? width : (i / (n - 1)) * width);
  const y = (v: number) => height - (Math.min(v, yMax) / yMax) * height;
  const base = new Array<number>(n).fill(0);
  return series.map((values) => {
    const top = values.map((v, i) => (base[i] ?? 0) + Math.max(0, v));
    const upper = top.map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`);
    const lower = base.map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`).reverse();
    top.forEach((v, i) => {
      base[i] = v;
    });
    return `M${upper.join("L")}L${lower.join("L")}Z`;
  });
}

/** The stack's top: the largest column sum. */
export function stackMax(series: readonly (readonly number[])[]): number {
  const n = series[0]?.length ?? 0;
  let max = 0;
  for (let i = 0; i < n; i++) {
    let sum = 0;
    for (const s of series) sum += Math.max(0, s[i] ?? 0);
    max = Math.max(max, sum);
  }
  return max;
}

/** Each week's parts as shares of its total, 0–100; an empty week is all zeros. */
export function mixShares(trend: MixTrend): number[][] {
  return trend.families.map((_, f) =>
    trend.weeks.map((w) => (w.total > 0 ? ((w.parts[f] ?? 0) / w.total) * 100 : 0)),
  );
}

/** The CSS colour of a model family (design-language.md: colour follows the family). */
export function familyColor(family: ModelFamily | null): string {
  switch (family) {
    case "opus":
      return "var(--series-2)";
    case "fable":
      return "var(--series-3)";
    case "sonnet":
      return "var(--series-4)";
    case "haiku":
      return "var(--series-5)";
    default:
      return "var(--text-3)";
  }
}

/** A strip block's left edge and width in percent of a day `minutes` long. */
export function blockBox(
  from: number,
  to: number,
  minutes: number,
): { left: number; width: number } {
  const span = Math.max(1, minutes);
  return { left: (from / span) * 100, width: (Math.max(0, to - from) / span) * 100 };
}

/** Contribution series ready to stack: a missing value repeats the last one (running totals). */
export function contribSeries(values: readonly (readonly (number | null)[])[]): number[][] {
  return values.map((series) => {
    let last = 0;
    return series.map((v) => {
      if (v != null) last = v;
      return last;
    });
  });
}

/** Colours of the contribution chart: the named sessions in fixed order, then the rest. */
export const CONTRIB_COLORS = [
  "var(--series-2)",
  "var(--series-3)",
  "var(--series-4)",
  "var(--series-5)",
];

export function contribColor(index: number, named: boolean): string {
  return named ? (CONTRIB_COLORS[index] ?? "var(--text-3)") : "var(--text-3)";
}

/** The point nearest to `x` (0–1 across the plot) among `n` evenly spaced ones. */
export function nearestIndex(x: number, n: number): number {
  if (n <= 1) return 0;
  return Math.max(0, Math.min(n - 1, Math.round(x * (n - 1))));
}
