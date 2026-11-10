// Synthetic panel snapshots for tests. Shaped like the backend's, never real usage.

import type {
  ChartPoint,
  FocusView,
  LimitView,
  PaceView,
  PanelDetail,
  PanelSnapshot,
  SessionShareView,
} from "../../lib/ipc/bindings";
import { limit, weekly } from "../gallery/fixtures";

export const NOW = 1_800_000_000_000;
const MIN = 60_000;

export function session(n: number, o: Partial<SessionShareView> = {}): SessionShareView {
  return {
    session_id: n,
    session_uuid: null,
    name: `session-${n}`,
    title: null,
    project: "demo",
    status: "busy",
    share_percent: 10 - n,
    cost: 0,
    tokens_per_min: 1_000,
    ...o,
  };
}

export function snapshot(o: Partial<PanelSnapshot> = {}): PanelSnapshot {
  return {
    account: {
      id: 1,
      label: "Claude",
      plan: "max",
      tier: "default_claude_max_20x",
      extra_usage_enabled: false,
    },
    limits: [limit(42, { projected: 68 }), weekly(34)],
    absent: [],
    live_sessions: [session(1), session(2)],
    top_sessions: [],
    index_state: {
      initial_scan_done: true,
      last_indexed_at: NOW,
      progress: null,
      recent_ready: true,
      drift: null,
      unpriced_models: [],
    },
    api: {
      state: { state: "active" },
      diagnostics: {
        requests: 3,
        errors: 0,
        last_error: null,
        last_error_at: null,
        last_attempt: NOW - 5 * MIN,
        last_success: NOW - 5 * MIN,
      },
    },
    statusline: { last_seen: NOW },
    generated_at: NOW,
    ...o,
  };
}

/** `view` with its projection replaced. */
export function projected(
  view: LimitView,
  p: Partial<NonNullable<LimitView["projection"]>>,
): LimitView {
  return {
    ...view,
    projection: {
      rate_per_min: 0.2,
      eta_100_at: NOW + 200 * MIN,
      will_hit_before_reset: false,
      projected_at_reset: 60,
      confidence: "high",
      method: "observed",
      ...p,
    },
  };
}

const HOUR = 60 * MIN;

/**
 * A detail view for `snap` as the core would build it: the pace of each measured limit and,
 * for the weekly limit, its 24-hour marks and a curve rising by day. Timed from `Date.now()`
 * like the gallery's limits.
 */
export function detailOf(snap: PanelSnapshot, readings = true): PanelDetail {
  const now = Date.now();
  const length = (kind: LimitView["kind"]) => (kind === "session" ? 5 * HOUR : 7 * 24 * HOUR);
  const pace: PaceView[] = snap.limits
    .filter((l) => l.source !== "estimate" && l.resets_at != null && l.resets_at > now)
    .map((l) => {
      const elapsed = (1 - ((l.resets_at ?? now) - now) / length(l.kind)) * 100;
      const ahead = (l.percent ?? 0) - elapsed;
      return {
        kind: l.kind,
        scope: l.scope,
        percent: l.percent,
        elapsed_percent: elapsed,
        ahead,
        warn: elapsed >= 100 / 7 && ahead >= 10,
      };
    });
  const weekly = snap.limits.find((l) => l.kind === "weekly_all" && l.resets_at != null);
  let focus: FocusView | null = null;
  if (weekly?.resets_at != null && weekly.percent != null) {
    const start = weekly.resets_at - 7 * 24 * HOUR;
    const points: ChartPoint[] = [];
    if (readings) {
      for (let at = start; at <= now; at += 6 * HOUR) {
        const share = (at - start) / (now - start);
        points.push({
          at,
          percent: Math.round(weekly.percent * share * 10) / 10,
          source: "statusline",
        });
      }
    }
    focus = {
      kind: "weekly_all",
      scope: null,
      window_start: start,
      resets_at: weekly.resets_at,
      periods: [1, 2, 3, 4, 5, 6].map((k) => ({
        at: start + k * 24 * HOUR,
        time_percent: (k / 7) * 100,
      })),
      points,
    };
  }
  return { pace, focus, generated_at: now };
}
