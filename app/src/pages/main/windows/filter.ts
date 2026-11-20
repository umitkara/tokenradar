// The windows and patterns pages' filters in the URL fragment (Step 17a): `#windows?tab=list
// &range=all`, `#patterns?range=4w&metric=percent`. Same rules as the sessions list
// (`sessions/filter.ts`): only non-default values are written, in a fixed order, and anything
// unreadable falls back to its default.

import type { HeatMetric, TimeRange } from "../../../lib/ipc/bindings";
import { day, formatDay, localDay, oneOf, queryText } from "../sessions/filter";

export type SpanPreset = "4w" | "12w" | "all" | "custom";
export const SPAN_PRESETS: readonly SpanPreset[] = ["4w", "12w", "all", "custom"];

export interface SpanQuery {
  range: SpanPreset;
  /** Local days `YYYY-MM-DD`, both inclusive; only with `range: "custom"`. */
  from: string | null;
  to: string | null;
}

const DAY_MS = 86_400_000;

/** The backend range for `now`; "all" starts at the epoch. */
export function spanRange(q: SpanQuery, now: Date): TimeRange {
  const end = now.getTime();
  switch (q.range) {
    case "4w":
      return { from: end - 28 * DAY_MS, to: end };
    case "12w":
      return { from: end - 84 * DAY_MS, to: end };
    case "custom": {
      const from = q.from ? localDay(q.from) : null;
      const to = q.to ? localDay(q.to) : null;
      if (from && to) {
        to.setDate(to.getDate() + 1);
        return { from: from.getTime(), to: to.getTime() };
      }
      return { from: 0, to: end };
    }
    case "all":
      return { from: 0, to: end };
  }
}

export function parseSpan(p: URLSearchParams, fallback: SpanPreset): SpanQuery {
  const range = oneOf(p.get("range"), SPAN_PRESETS) ?? fallback;
  const from = range === "custom" ? day(p.get("from")) : null;
  const to = range === "custom" ? day(p.get("to")) : null;
  const days = from != null && to != null ? [from, to].sort() : null;
  return {
    range: range === "custom" && days == null ? fallback : range,
    from: days?.[0] ?? null,
    to: days?.[1] ?? null,
  };
}

export function writeSpan(p: URLSearchParams, q: SpanQuery, fallback: SpanPreset) {
  const custom = q.range === "custom" && q.from != null && q.to != null;
  if (q.range !== fallback && (q.range !== "custom" || custom)) p.set("range", q.range);
  if (custom && q.from && q.to) {
    p.set("from", q.from);
    p.set("to", q.to);
  }
}

/** A custom range's starting days when the user switches to it: the last four weeks. */
export function customDefault(q: SpanQuery, now: Date): Pick<SpanQuery, "from" | "to"> {
  const start = new Date(now);
  start.setDate(now.getDate() - 27);
  return { from: q.from ?? formatDay(start), to: q.to ?? formatDay(now) };
}

export type WindowsTab = "weeks" | "strip" | "list";
export const WINDOWS_TABS: readonly WindowsTab[] = ["weeks", "strip", "list"];

export interface WindowsQuery extends SpanQuery {
  tab: WindowsTab;
}

export const DEFAULT_WINDOWS_QUERY: WindowsQuery = {
  tab: "weeks",
  range: "12w",
  from: null,
  to: null,
};

export function parseWindowsQuery(text: string): WindowsQuery {
  const p = new URLSearchParams(text);
  return {
    ...parseSpan(p, DEFAULT_WINDOWS_QUERY.range),
    tab: oneOf(p.get("tab"), WINDOWS_TABS) ?? DEFAULT_WINDOWS_QUERY.tab,
  };
}

export function toWindowsQuery(q: WindowsQuery): string {
  const p = new URLSearchParams();
  if (q.tab !== DEFAULT_WINDOWS_QUERY.tab) p.set("tab", q.tab);
  writeSpan(p, q, DEFAULT_WINDOWS_QUERY.range);
  return queryText(p);
}

export const HEAT_METRICS: readonly HeatMetric[] = ["percent", "cost", "tokens"];

export interface PatternsQuery extends SpanQuery {
  metric: HeatMetric;
}

export const DEFAULT_PATTERNS_QUERY: PatternsQuery = {
  range: "12w",
  from: null,
  to: null,
  metric: "cost",
};

export function parsePatternsQuery(text: string): PatternsQuery {
  const p = new URLSearchParams(text);
  return {
    ...parseSpan(p, DEFAULT_PATTERNS_QUERY.range),
    metric: oneOf(p.get("metric"), HEAT_METRICS) ?? DEFAULT_PATTERNS_QUERY.metric,
  };
}

export function toPatternsQuery(q: PatternsQuery): string {
  const p = new URLSearchParams();
  writeSpan(p, q, DEFAULT_PATTERNS_QUERY.range);
  if (q.metric !== DEFAULT_PATTERNS_QUERY.metric) p.set("metric", q.metric);
  return queryText(p);
}

/** The windows page's last query text, for "Back" from a window's drawer. */
export const windowsMemory = { query: "" };
