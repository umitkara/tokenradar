// Formatting steps of the sessions pages (Step 16a). Values come finished from the backend
// (`list_sessions`, `get_session`); these only shorten names, label models, choose which share a
// row shows and lay out the detail's chart and tables.

import { type ChartBand, timeTicks } from "../../../lib/chart/model";
import type {
  Basis,
  ImpactRow,
  ModelFamily,
  SessionRow,
  TimelineBucket,
} from "../../../lib/ipc/bindings";

/**
 * A long name shortened in the middle, so both the start and the distinguishing end stay
 * visible: `refactor the indexer … retention watermark`. Counts characters, not pixels; the cell
 * still clips with an end ellipsis if the font runs wide.
 */
export function middleEllipsis(text: string, max = 64): string {
  const chars = [...text];
  if (chars.length <= max) return text;
  const keep = max - 1;
  const head = Math.ceil(keep / 2);
  const tail = keep - head;
  return `${chars.slice(0, head).join("").trimEnd()}…${chars
    .slice(chars.length - tail)
    .join("")
    .trimStart()}`;
}

/**
 * A model id as a badge: `claude-opus-5-5` → `Opus 5.5`, `claude-haiku-4-5-20251001` →
 * `Haiku 4.5`. Ids that do not follow the pattern are shown without the `claude-` prefix.
 */
export function modelLabel(id: string): string {
  const name = id.replace(/^claude-/, "");
  const m = /^([a-z]+)-(\d+)(?:-(\d{1,2}))?(?:-\d{8})?$/.exec(name);
  if (!m?.[1] || !m[2]) return name;
  const family = m[1].charAt(0).toUpperCase() + m[1].slice(1);
  return `${family} ${m[2]}${m[3] ? `.${m[3]}` : ""}`;
}

/**
 * The share a row shows in its "Impact" column: the weekly limit's (always `≈`, attribution is an
 * estimate), or the 5-hour windows' when the account has no weekly share for it. `kind` names
 * which, for the column's tooltip.
 */
export function impactOf(
  row: Pick<SessionRow, "weekly_percent" | "session_percent" | "impact_basis">,
): { kind: "weekly" | "session"; percent: number; dim: boolean } | null {
  const dim = row.impact_basis?.dim ?? true;
  if (row.weekly_percent != null) return { kind: "weekly", percent: row.weekly_percent, dim };
  if (row.session_percent != null) return { kind: "session", percent: row.session_percent, dim };
  return null;
}

/** How long a session ran; `null` without both ends. */
export function durationOf(row: Pick<SessionRow, "first_ts" | "last_ts">): number | null {
  if (row.first_ts == null || row.last_ts == null) return null;
  return Math.max(0, row.last_ts - row.first_ts);
}

/** A stack of the detail's cost chart: a model family, or `other` for the rest. */
export type StackKey = "opus" | "fable" | "sonnet" | "haiku" | "other";

/**
 * Stacks in a fixed order, bottom up, each with a fixed color token: color follows the family,
 * never its rank, so Opus is the same blue on every session (validated palette, ADR-068).
 */
export const STACKS: readonly { key: StackKey; token: string }[] = [
  { key: "opus", token: "--series-2" },
  { key: "fable", token: "--series-3" },
  { key: "sonnet", token: "--series-4" },
  { key: "haiku", token: "--series-5" },
  { key: "other", token: "--text-3" },
];

export function stackOf(family: ModelFamily): StackKey {
  return family === "opus" || family === "fable" || family === "sonnet" || family === "haiku"
    ? family
    : "other";
}

export interface CostStack {
  key: StackKey;
  token: string;
  /** Model ids in this stack, most expensive first. */
  models: string[];
  /** Micro-USD per bucket, `0` where the stack was idle. */
  values: number[];
}

export interface CostChart {
  /** Bucket starts, Unix ms, oldest first. */
  xs: number[];
  /** Only the stacks with any cost, in `STACKS` order. */
  stacks: CostStack[];
  bucketMs: number;
}

/** The detail's timeline as aligned stacks: one x per bucket with any cost. */
export function costChart(timeline: readonly TimelineBucket[], bucketMs: number): CostChart {
  const xs = [...new Set(timeline.map((b) => b.at))].sort((a, b) => a - b);
  const index = new Map(xs.map((x, i) => [x, i]));
  const byKey = new Map<StackKey, { values: number[]; models: Map<string, number> }>();
  for (const b of timeline) {
    const i = index.get(b.at);
    if (i === undefined) continue;
    const key = stackOf(b.family);
    let s = byKey.get(key);
    if (!s) {
      s = { values: xs.map(() => 0), models: new Map() };
      byKey.set(key, s);
    }
    s.values[i] = (s.values[i] ?? 0) + b.cost;
    s.models.set(b.model, (s.models.get(b.model) ?? 0) + b.cost);
  }
  const stacks = STACKS.flatMap(({ key, token }) => {
    const s = byKey.get(key);
    if (!s) return [];
    const models = [...s.models.entries()]
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .map(([m]) => m);
    return [{ key, token, models, values: s.values }];
  });
  return { xs, stacks, bucketMs };
}

/**
 * Background bands of the cost chart from the impact rows: each unscoped window the session
 * touched, the ones that reached their limit tinted. Weekly windows only when there is no 5-hour
 * window (the 5-hour boundaries are the finer grid).
 */
export function windowBands(impact: readonly ImpactRow[]): ChartBand[] {
  const unscoped = impact.filter((r) => r.scope == null);
  const five = unscoped.filter((r) => r.kind === "session");
  const rows = five.length > 0 ? five : unscoped.filter((r) => r.kind === "weekly_all");
  return rows
    .map((r) => ({
      from: r.starts_at,
      to: r.resets_at,
      kind: r.hit_limit ? "hit" : r.estimated ? "estimated" : "window",
    }))
    .sort((a, b) => a.from - b.from) as ChartBand[];
}

/** The impact table's order: 5-hour windows first, then weekly, then scoped; newest first. */
export function impactRows(impact: readonly ImpactRow[]): ImpactRow[] {
  const rank = (r: ImpactRow) => (r.scope != null ? 2 : r.kind === "session" ? 0 : 1);
  return [...impact].sort((a, b) => rank(a) - rank(b) || b.resets_at - a.resets_at);
}

/** How a share is written: `≈` unless split with a usable calibration, dimmed when weak. */
export function shareStyle(basis: Basis | null): { approx: boolean; dim: boolean } {
  return { approx: basis?.approx ?? true, dim: basis?.dim ?? true };
}

/**
 * The cost axis: a round top at or above `max` (1, 2 or 5 times a power of ten) and its step,
 * four steps tall. `max <= 0` gives a unit axis so an empty chart still has a scale.
 */
export function costAxis(max: number): { top: number; step: number } {
  if (!(max > 0)) return { top: 4, step: 1 };
  const raw = max / 4;
  const pow = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 5, 10].map((m) => m * pow).find((s) => s >= raw) ?? 10 * pow;
  return { top: step * 4, step };
}

/** The highest stacked bar: the sum of all stacks per bucket. */
export function stackMax(chart: CostChart): number {
  let max = 0;
  for (let i = 0; i < chart.xs.length; i++) {
    let sum = 0;
    for (const s of chart.stacks) sum += s.values[i] ?? 0;
    max = Math.max(max, sum);
  }
  return max;
}

const MINUTE = 60_000;

/**
 * X ticks for the cost chart's span: the overview's day and hour ticks from six hours up, below
 * that whole 10, 15, 30 or 60 minutes so a short session still gets three to six labels.
 */
export function costTicks(from: number, to: number): number[] {
  const span = to - from;
  if (span >= 6 * 60 * MINUTE) return timeTicks(from, to);
  const step = ([10, 15, 30, 60].find((m) => span / (m * MINUTE) <= 6) ?? 60) * MINUTE;
  const out: number[] = [];
  const d = new Date(from);
  d.setSeconds(0, 0);
  const stepMin = step / MINUTE;
  d.setMinutes(Math.ceil(d.getMinutes() / stepMin) * stepMin);
  for (let at = d.getTime(); at <= to; at += step) if (at >= from) out.push(at);
  return out;
}
