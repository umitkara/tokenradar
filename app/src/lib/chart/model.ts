// What the chart wrapper draws, and the pure steps from it to uPlot's arrays. The backend sends
// finished values (thinned with LTTB, fused, with their source); these functions only lay them
// out: one shared x axis, a solid and a dashed series per line, bands and axis labels.

import { clock, type Locale, weekdayName } from "../i18n/format";

/** A line's color role: the 5-hour window, the weekly limit, the "Other" bucket. */
export type Tone = "primary" | "secondary" | "other";

export interface ChartPoint {
  /** Unix milliseconds. */
  at: number;
  value: number;
  /** Drawn dashed (design-language.md §2: estimates are dashed and carry "≈"). */
  estimated?: boolean;
}

export interface ChartLine {
  id: string;
  label: string;
  tone: Tone;
  /** Oldest first. */
  points: ChartPoint[];
}

/**
 * A vertical band: `window` alternates faintly to show limit window boundaries, `hit` is a
 * window that reached its limit (critical tint), `estimated` a window without readings.
 */
export interface ChartBand {
  from: number;
  to: number;
  kind: "window" | "hit" | "estimated";
}

/**
 * A span drawn in a lane under the plot, on the same time axis: a session from its start to its
 * end. Hover or focus names it and gives its share.
 */
export interface ChartSpan {
  id: string;
  from: number;
  to: number;
  label: string;
  /** `≈` points of the limit; `null` without a percentage. */
  percent: number | null;
}

export interface ChartModel {
  /** The x range, Unix milliseconds. */
  from: number;
  to: number;
  /** Top of the y axis (100 for percentages). */
  yMax: number;
  lines: ChartLine[];
  bands: ChartBand[];
  /** Faint dashed horizontal lines (75 and 90). */
  thresholds: number[];
  /** Session spans in a lane under the plot, one row each. */
  spans?: ChartSpan[];
}

/**
 * Where a span sits in a plot `width` px wide showing `[from, to]`: left offset and width in px,
 * clipped to the plot, at least `min` px wide so a short session stays visible and hoverable.
 * `null` when it lies outside the range.
 */
export function spanBox(
  span: { from: number; to: number },
  from: number,
  to: number,
  width: number,
  min = 6,
): { left: number; width: number } | null {
  if (to <= from || span.to < from || span.from > to) return null;
  const x = (t: number) => ((Math.min(Math.max(t, from), to) - from) / (to - from)) * width;
  const x0 = x(span.from);
  const w = Math.max(min, x(span.to) - x0);
  return { left: Math.min(x0, Math.max(0, width - w)), width: Math.min(w, width) };
}

export interface AlignedSeries {
  lineId: string;
  tone: Tone;
  dashed: boolean;
  /** `undefined`: no sample at this x (uPlot joins across it); `null`: a break. */
  values: (number | null | undefined)[];
}

export interface Aligned {
  xs: number[];
  series: AlignedSeries[];
}

/**
 * One shared, sorted x axis for every line, and per line a solid and a dashed series. Where the
 * line has no sample (another line's x) the value is `undefined`, which uPlot joins across;
 * where the other kind draws it is `null`, a break. Where the kind changes, the boundary point
 * is in both so the line stays connected. A line with no estimates gets no dashed series, and
 * the reverse.
 */
export function align(lines: readonly ChartLine[]): Aligned {
  const xs = [...new Set(lines.flatMap((l) => l.points.map((p) => p.at)))].sort((a, b) => a - b);
  const index = new Map(xs.map((x, i) => [x, i]));
  const series: AlignedSeries[] = [];
  for (const line of lines) {
    const solid: (number | null | undefined)[] = xs.map(() => undefined);
    const dashed: (number | null | undefined)[] = xs.map(() => undefined);
    let hasSolid = false;
    let hasDashed = false;
    line.points.forEach((p, k) => {
      const i = index.get(p.at);
      if (i === undefined) return;
      const target = p.estimated ? dashed : solid;
      const other = p.estimated ? solid : dashed;
      target[i] = p.value;
      if (other[i] === undefined) other[i] = null;
      if (p.estimated) hasDashed = true;
      else hasSolid = true;
      // Join the two kinds at the boundary: the previous point also starts this kind's run.
      const prev = line.points[k - 1];
      if (prev && Boolean(prev.estimated) !== Boolean(p.estimated)) {
        const j = index.get(prev.at);
        if (j !== undefined) target[j] = prev.value;
      }
    });
    if (hasSolid) series.push({ lineId: line.id, tone: line.tone, dashed: false, values: solid });
    if (hasDashed) series.push({ lineId: line.id, tone: line.tone, dashed: true, values: dashed });
  }
  return { xs, series };
}

/** The value of `line` nearest at or before `at`, for the tooltip; `null` before its first. */
export function valueAt(line: ChartLine, at: number): ChartPoint | null {
  let found: ChartPoint | null = null;
  for (const p of line.points) {
    if (p.at > at) break;
    found = p;
  }
  return found;
}

/** The latest point of a line, for the screen reader summary. */
export function latest(line: ChartLine): ChartPoint | null {
  return line.points[line.points.length - 1] ?? null;
}

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

const MONTHS: Record<Locale, string[]> = {
  en: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
  tr: ["Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara"],
};

/** A short date: TR `20 Kas`, EN `Nov 20`. Local time. */
export function shortDate(locale: Locale, ms: number): string {
  const d = new Date(ms);
  const month = MONTHS[locale][d.getMonth()] ?? "";
  return locale === "tr" ? `${d.getDate()} ${month}` : `${month} ${d.getDate()}`;
}

/**
 * Local midnights inside `[from, to]` for a range of two days or more, otherwise every 3 or 6
 * hours on the hour. These are the x ticks and gridlines.
 */
export function timeTicks(from: number, to: number): number[] {
  const span = to - from;
  const out: number[] = [];
  if (span >= 2 * DAY) {
    const d = new Date(from);
    d.setHours(0, 0, 0, 0);
    if (d.getTime() < from) d.setDate(d.getDate() + 1);
    // `setDate` keeps local midnight across daylight saving changes.
    for (; d.getTime() <= to; d.setDate(d.getDate() + 1)) out.push(d.getTime());
    return out;
  }
  const step = span > 12 * HOUR ? 6 : 3;
  const d = new Date(from);
  d.setMinutes(0, 0, 0);
  if (d.getTime() < from) d.setHours(d.getHours() + 1);
  while (d.getHours() % step !== 0) d.setHours(d.getHours() + 1);
  for (; d.getTime() <= to; d.setHours(d.getHours() + step)) out.push(d.getTime());
  return out;
}

/** An x tick label: weekday and date for day ticks (`Per 20`), the clock for hour ticks. */
export function tickLabel(locale: Locale, hour12: boolean, ms: number, span: number): string {
  const d = new Date(ms);
  if (span >= 2 * DAY) return `${weekdayName(locale, d.getDay())} ${d.getDate()}`;
  return clock(locale, d.getHours(), d.getMinutes(), hour12);
}

/** The tooltip's time: TR `Per 20 Kas 14:05`, EN `Thu Nov 20 2:05 PM`. */
export function tooltipTime(locale: Locale, hour12: boolean, ms: number): string {
  const d = new Date(ms);
  return `${weekdayName(locale, d.getDay())} ${shortDate(locale, ms)} ${clock(
    locale,
    d.getHours(),
    d.getMinutes(),
    hour12,
  )}`;
}
