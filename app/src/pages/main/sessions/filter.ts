// The sessions list's filters and their place in the URL fragment (Step 16a): `#sessions?range=7d
// &project=12&model=opus&live=1&q=text&sort=cost`. Kept in the fragment so back and forward step
// through filter changes and a link reopens the same list. Only non-default values are written,
// in a fixed order, so the same filters always give the same text. Unknown or malformed values
// fall back to their default instead of failing.

import type { ModelFamily, SessionFilter, SessionSort, TimeRange } from "../../../lib/ipc/bindings";

export type RangePreset = "today" | "7d" | "30d" | "all" | "custom";

export const RANGE_PRESETS: readonly RangePreset[] = ["today", "7d", "30d", "all", "custom"];
export const SORTS: readonly SessionSort[] = ["last_active", "started", "cost", "tokens", "impact"];
/** The families a person can pick; `synthetic` and `unknown` are not models one works with. */
export const FAMILIES: readonly ModelFamily[] = ["fable", "mythos", "opus", "sonnet", "haiku"];

export interface SessionQuery {
  range: RangePreset;
  /** Local calendar days `YYYY-MM-DD`, both inclusive; only with `range: "custom"`. */
  from: string | null;
  to: string | null;
  project: number | null;
  family: ModelFamily | null;
  live: boolean;
  /** Search text as typed (trimmed when queried). */
  q: string;
  sort: SessionSort;
}

export const DEFAULT_QUERY: SessionQuery = {
  range: "30d",
  from: null,
  to: null,
  project: null,
  family: null,
  live: false,
  q: "",
  sort: "last_active",
};

const DAY_MS = 86_400_000;
const DAY_RE = /^\d{4}-\d{2}-\d{2}$/;

export function day(s: string | null): string | null {
  if (s == null || !DAY_RE.test(s)) return null;
  const d = localDay(s);
  return d && formatDay(d) === s ? s : null;
}

/** `YYYY-MM-DD` → local midnight of that day; `null` when it is not a real date. */
export function localDay(s: string): Date | null {
  const [y, m, d] = s.split("-").map(Number);
  if (!y || !m || !d) return null;
  const date = new Date(y, m - 1, d);
  return Number.isNaN(date.getTime()) ? null : date;
}

export function formatDay(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

export function oneOf<T extends string>(value: string | null, allowed: readonly T[]): T | null {
  return value != null && (allowed as readonly string[]).includes(value) ? (value as T) : null;
}

/** The fragment's query text (without `?`) → filters; anything unreadable is the default. */
export function parseQuery(text: string): SessionQuery {
  const p = new URLSearchParams(text);
  const project = Number(p.get("project"));
  return {
    ...parseRange(p, DEFAULT_QUERY.range),
    project: Number.isSafeInteger(project) && project > 0 ? project : null,
    family: oneOf(p.get("model"), FAMILIES),
    live: p.get("live") === "1",
    q: p.get("q") ?? "",
    sort: oneOf(p.get("sort"), SORTS) ?? DEFAULT_QUERY.sort,
  };
}

/** Filters → the fragment's query text, defaults left out; `""` for the default list. */
export function toQuery(f: SessionQuery): string {
  const p = new URLSearchParams();
  writeRange(p, f, DEFAULT_QUERY.range);
  if (f.project != null) p.set("project", String(f.project));
  if (f.family != null) p.set("model", f.family);
  if (f.live) p.set("live", "1");
  if (f.q.trim() !== "") p.set("q", f.q);
  if (f.sort !== DEFAULT_QUERY.sort) p.set("sort", f.sort);
  return queryText(p);
}

/** URLSearchParams writes spaces as `+`; `%20` reads the same and survives copy and paste. */
export function queryText(p: URLSearchParams): string {
  return p.toString().replace(/\+/g, "%20");
}

/** A range preset and its custom days, as the sessions and projects lists keep them. */
export interface RangeQuery {
  range: RangePreset;
  from: string | null;
  to: string | null;
}

/**
 * The backend range for `now`: presets count back from now ("today" from local midnight), a
 * custom range covers its local days whole; `null` for all time.
 */
export function rangeFilter(f: RangeQuery, now: Date): TimeRange | null {
  const end = now.getTime();
  switch (f.range) {
    case "today": {
      const start = new Date(now);
      start.setHours(0, 0, 0, 0);
      return { from: start.getTime(), to: end };
    }
    case "7d":
      return { from: end - 7 * DAY_MS, to: end };
    case "30d":
      return { from: end - 30 * DAY_MS, to: end };
    case "custom": {
      const from = f.from ? localDay(f.from) : null;
      const to = f.to ? localDay(f.to) : null;
      if (!from || !to) return null;
      // The day after `to`, at local midnight (`setDate` keeps it across DST changes).
      to.setDate(to.getDate() + 1);
      return { from: from.getTime(), to: to.getTime() };
    }
    case "all":
      return null;
  }
}

/** Reads `range`, `from` and `to`; a custom range needs both days, earlier first. */
export function parseRange(p: URLSearchParams, fallback: RangePreset): RangeQuery {
  const range = oneOf(p.get("range"), RANGE_PRESETS) ?? fallback;
  const from = range === "custom" ? day(p.get("from")) : null;
  const to = range === "custom" ? day(p.get("to")) : null;
  // ISO days compare as text.
  const days = from != null && to != null ? [from, to].sort() : null;
  return {
    range: range === "custom" && days == null ? fallback : range,
    from: days?.[0] ?? null,
    to: days?.[1] ?? null,
  };
}

/** Writes `range`, `from` and `to` unless the range is `fallback`. */
export function writeRange(p: URLSearchParams, f: RangeQuery, fallback: RangePreset) {
  const custom = f.range === "custom" && f.from != null && f.to != null;
  if (f.range !== fallback && (f.range !== "custom" || custom)) p.set("range", f.range);
  if (custom && f.from && f.to) {
    p.set("from", f.from);
    p.set("to", f.to);
  }
}

/** The backend filter for `now`. `locked` pins the project (a project's detail). */
export function toFilter(f: SessionQuery, now: Date, locked: number | null = null): SessionFilter {
  const search = f.q.trim();
  return {
    range: rangeFilter(f, now),
    project: locked ?? f.project,
    family: f.family,
    live_only: f.live,
    search: search === "" ? null : search,
  };
}

/**
 * The list's last query text, so "Back to sessions" from a detail returns to the same filters
 * even when the detail was opened by a link rather than from the list.
 */
export const listMemory = { query: "" };
