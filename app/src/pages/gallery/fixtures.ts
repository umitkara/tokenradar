// Synthetic view models for the gallery. Shaped like the backend's, never real usage.

import type {
  LimitKind,
  LimitView,
  Projection,
  SessionShareView,
  Severity,
  Source,
} from "../../lib/ipc/bindings";

const MIN = 60_000;
const HOUR = 60 * MIN;

interface LimitOptions {
  kind?: LimitKind;
  model?: string;
  severity?: Severity;
  source?: Source;
  projected?: number;
  resetsIn?: number;
  stale?: boolean;
}

function projection(projected: number | undefined): Projection | null {
  if (projected == null) return null;
  return {
    rate_per_min: 0.2,
    eta_100_at: null,
    will_hit_before_reset: projected > 100,
    projected_at_reset: projected,
    confidence: "medium",
    method: "observed",
  };
}

const defaultReset = (kind: LimitKind) => (kind === "session" ? 108 * MIN : 3 * 24 * HOUR);

export function limit(percent: number | null, o: LimitOptions = {}): LimitView {
  const now = Date.now();
  const source = o.source ?? "statusline";
  const kind = o.kind ?? "session";
  return {
    kind,
    scope: o.model ? { model: o.model } : null,
    percent,
    // Half a minute of slack so "1h 48m" holds while a test or screenshot runs.
    resets_at: percent == null ? null : now + 30_000 + (o.resetsIn ?? defaultReset(kind)),
    severity: o.severity ?? "ok",
    provider_severity: null,
    source,
    estimate_reason: null,
    observed_at: now - (o.stale ? 45 * MIN : 2 * MIN),
    freshness: o.stale ? "stale" : "fresh",
    projection: source === "estimate" ? null : projection(o.projected),
  };
}

export const weekly = (percent: number | null, o: LimitOptions = {}) =>
  limit(percent, { kind: "weekly_all", ...o });

export interface RingCase {
  key: string;
  inner: LimitView | null;
  outer: LimitView | null;
}

export const ringCases: RingCase[] = [
  { key: "gallery-case-ok", inner: limit(42, { projected: 61 }), outer: weekly(18) },
  {
    key: "gallery-case-warn",
    inner: limit(81, { severity: "warn", projected: 96 }),
    outer: weekly(52, { projected: 70 }),
  },
  {
    key: "gallery-case-crit",
    inner: limit(94, { severity: "crit", resetsIn: 23 * MIN }),
    outer: weekly(91, { severity: "crit" }),
  },
  {
    key: "gallery-case-over",
    inner: limit(70, { projected: 130, resetsIn: 3 * HOUR }),
    outer: weekly(34, { projected: 58 }),
  },
  {
    key: "gallery-case-estimate",
    inner: limit(35, { source: "estimate" }),
    outer: weekly(22, { source: "estimate" }),
  },
  { key: "gallery-case-empty", inner: null, outer: null },
];

export const bars: LimitView[] = [
  limit(72, { projected: 88 }),
  weekly(34, { projected: 51 }),
  weekly(81, { kind: "weekly_scoped", model: "Fable", severity: "warn" }),
  limit(93, { severity: "crit", projected: 120, resetsIn: 40 * MIN }),
  limit(38, { source: "estimate" }),
  weekly(12, { stale: true, source: "api" }),
];

export const sessions: SessionShareView[] = [
  {
    session_id: 1,
    session_uuid: null,
    name: "refactor-indexer",
    title: null,
    project: "tokenradar",
    status: "busy",
    share_percent: 9,
    cost: 0,
    tokens_per_min: 12_400,
  },
  {
    session_id: 2,
    session_uuid: null,
    name: null,
    title: "Fix flaky watcher test",
    project: "atlas",
    status: "idle",
    share_percent: 3.5,
    cost: 0,
    tokens_per_min: 800,
  },
  {
    session_id: 3,
    session_uuid: null,
    name: null,
    title: null,
    project: "scratch",
    status: null,
    share_percent: null,
    cost: 0,
    tokens_per_min: null,
  },
];

/** Moves every value a little: the gallery's "Change values" shows transitions and the sweep. */
export function nudge(view: LimitView): LimitView {
  if (view.percent == null) return view;
  const percent = Math.max(0, Math.min(100, view.percent + Math.round(Math.random() * 30 - 12)));
  const severity: Severity = percent >= 90 ? "crit" : percent >= 75 ? "warn" : "ok";
  return { ...view, percent, severity: view.source === "estimate" ? "ok" : severity };
}
