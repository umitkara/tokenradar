// Synthetic panel snapshots for tests. Shaped like the backend's, never real usage.

import type { LimitView, PanelSnapshot, SessionShareView } from "../../lib/ipc/bindings";
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
    live_sessions: [session(1), session(2)],
    top_sessions: [],
    index_state: {
      initial_scan_done: true,
      last_indexed_at: NOW,
      progress: null,
      recent_ready: true,
      drift: null,
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
