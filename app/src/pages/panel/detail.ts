// The detail view's layout (ADR-048): instants to positions, the calendar days of the time
// strip and the chart's paths. Every percentage, pace and 24-hour mark comes from the core's
// `PanelDetail`; this only places them. Local midnights are worked out here, next to the day
// names they separate: they are calendar presentation in the system's time zone, and `Date`
// handles a daylight-saving change inside the window.

import type { ChartPoint, SessionShareView } from "../../lib/ipc/bindings";

const HOUR_MS = 3_600_000;

/** Where `at` falls in `start..end`, 0–100, clamped. */
export function position(at: number, start: number, end: number): number {
  if (end <= start) return 0;
  return Math.min(100, Math.max(0, ((at - start) / (end - start)) * 100));
}

/** Local midnights strictly inside `start..end`. */
export function localMidnights(start: number, end: number): number[] {
  const out: number[] = [];
  const d = new Date(start);
  d.setHours(0, 0, 0, 0);
  for (;;) {
    d.setDate(d.getDate() + 1);
    d.setHours(0, 0, 0, 0);
    const at = d.getTime();
    if (at >= end) break;
    if (at > start) out.push(at);
  }
  return out;
}

/** Whole local hours strictly inside `start..end`: the 5-hour chart's gridlines. */
export function localHours(start: number, end: number): number[] {
  const out: number[] = [];
  const d = new Date(start);
  d.setMinutes(0, 0, 0);
  for (let at = d.getTime() + HOUR_MS; at < end; at += HOUR_MS) {
    if (at > start) out.push(at);
  }
  return out;
}

/** The gridlines of a window: midnights for a week, hours for the 5-hour window. */
export function gridlines(start: number, end: number): number[] {
  return end - start > 24 * HOUR_MS ? localMidnights(start, end) : localHours(start, end);
}

export interface StripDay {
  /** The day's first instant inside the window (the window's start for the first day). */
  at: number;
  /** Left and right edges, 0–100. */
  from: number;
  to: number;
  /** Whether the day is wide enough for its name. */
  label: boolean;
}

/**
 * Narrowest part of the strip, in percent, a day name fits into (~24 px of the panel's ~300).
 * A day with the reset hour ("Wed 10:59") would need more than a whole day's width, so the
 * hour is in the legend under the chart instead.
 */
const DAY_ROOM = 8;

/** The calendar days of the window, cut at local midnight: the first and last are partial. */
export function stripDays(start: number, end: number): StripDay[] {
  const starts = [start, ...localMidnights(start, end)];
  return starts.map((at, i) => {
    const from = position(at, start, end);
    const to = position(starts[i + 1] ?? end, start, end);
    return { at, from, to, label: to - from >= DAY_ROOM };
  });
}

/** Distance, in percent, a period value keeps from the "now" mark. */
const NOW_ROOM = 10;

/** Whether a 24-hour mark at `pos` leaves room for the "now" mark at `nowPos`. */
export function clearOfNow(pos: number, nowPos: number): boolean {
  return Math.abs(pos - nowPos) >= NOW_ROOM;
}

export interface ChartGeometry {
  width: number;
  height: number;
  /** Top padding, so a 100 % curve keeps its stroke. */
  top: number;
}

export const CHART: ChartGeometry = { width: 300, height: 96, top: 4 };

export function chartX(at: number, start: number, end: number, g = CHART): number {
  return (position(at, start, end) / 100) * g.width;
}

export function chartY(percent: number, g = CHART): number {
  const p = Math.min(100, Math.max(0, percent));
  return g.top + (1 - p / 100) * (g.height - g.top);
}

/** The usage curve and the area under it, as SVG path data; empty without points. */
export function usagePaths(
  points: readonly ChartPoint[],
  start: number,
  end: number,
  g = CHART,
): { line: string; area: string } {
  const pts = points.filter((p) => p.percent != null && p.at >= start && p.at <= end);
  const first = pts[0];
  const last = pts[pts.length - 1];
  if (first === undefined || last === undefined) return { line: "", area: "" };
  const f = (n: number) => n.toFixed(1);
  const line = pts
    .map(
      (p, i) =>
        `${i === 0 ? "M" : "L"}${f(chartX(p.at, start, end, g))},${f(chartY(p.percent ?? 0, g))}`,
    )
    .join(" ");
  const base = f(chartY(0, g));
  const area = `${line} L${f(chartX(last.at, start, end, g))},${base} L${f(chartX(first.at, start, end, g))},${base} Z`;
  return { line, area };
}

export interface SessionBar {
  /** 0–100. */
  width: number;
  /** A ≈ share (dashed), not a measured cost share. */
  estimate: boolean;
}

/**
 * The live sessions' bars, as in the mod (ADR-046): drawn only when there is something to
 * compare. The ≈ share of the window when the core knows it, otherwise the session's part of
 * the listed sessions' cost when at least two have one. `null` where nothing is drawn.
 */
export function sessionBars(rows: readonly SessionShareView[]): (SessionBar | null)[] {
  if (rows.some((r) => r.share_percent != null)) {
    return rows.map((r) =>
      r.share_percent == null
        ? null
        : { width: Math.min(100, Math.max(0, r.share_percent)), estimate: true },
    );
  }
  const costs = rows.filter((r) => r.cost > 0);
  if (costs.length < 2) return rows.map(() => null);
  const total = costs.reduce((sum, r) => sum + r.cost, 0);
  return rows.map((r) => (r.cost > 0 ? { width: (r.cost / total) * 100, estimate: false } : null));
}

/** Micro-dollars as dollars. */
export function usd(micro: number): number {
  return micro / 1_000_000;
}
