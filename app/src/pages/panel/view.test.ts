import { describe, expect, it } from "vitest";
import { limit, weekly } from "../gallery/fixtures";
import { NOW, projected, session, snapshot } from "./fixtures";
import {
  emptyState,
  limitsOf,
  notices,
  planLabel,
  projectionLine,
  refreshState,
  sessionList,
} from "./view";

const MIN = 60_000;

describe("limitsOf", () => {
  it("sorts limits into the ring's slots and the scoped rows", () => {
    const scoped = weekly(12, { kind: "weekly_scoped", model: "Fable" });
    const l = limitsOf(snapshot({ limits: [weekly(34), scoped, limit(42)] }));
    expect(l.fiveHour?.percent).toBe(42);
    expect(l.weekly?.percent).toBe(34);
    expect(l.scoped).toEqual([scoped]);
  });
});

describe("planLabel", () => {
  it("names the Max multiplier from the tier", () => {
    expect(planLabel(snapshot().account)).toBe("Max 20x");
  });
  it("falls back to the plan name, or nothing", () => {
    const account = { id: 1, label: "Claude", plan: "pro", tier: null, extra_usage_enabled: null };
    expect(planLabel(account)).toBe("Pro");
    expect(planLabel({ ...account, plan: null })).toBeNull();
    expect(planLabel(null)).toBeNull();
  });
});

describe("projectionLine", () => {
  it("warns when the 5-hour window fills before its reset", () => {
    const five = projected(limit(93, { severity: "crit" }), {
      will_hit_before_reset: true,
      eta_100_at: NOW + 38 * MIN,
      projected_at_reset: 240,
    });
    const line = projectionLine(limitsOf(snapshot({ limits: [five, weekly(34)] })));
    expect(line).toMatchObject({ type: "will-hit", kind: "five_hour", etaAt: NOW + 38 * MIN });
  });

  it("warns about the weekly limit when only it fills first", () => {
    const week = projected(weekly(91), {
      will_hit_before_reset: true,
      eta_100_at: NOW + 600 * MIN,
      confidence: "low",
    });
    const five = projected(limit(40), { projected_at_reset: 70 });
    const line = projectionLine(limitsOf(snapshot({ limits: [five, week] })));
    expect(line).toMatchObject({ type: "will-hit", kind: "weekly", low: true });
  });

  it("states the level at the reset otherwise", () => {
    const five = projected(limit(42), { projected_at_reset: 86, confidence: "low" });
    const line = projectionLine(limitsOf(snapshot({ limits: [five] })));
    expect(line).toEqual({ type: "at-reset", kind: "five_hour", pct: 86, low: true });
  });

  it("says so when usage stopped", () => {
    const five = projected(limit(42), { rate_per_min: 0, eta_100_at: null });
    expect(projectionLine(limitsOf(snapshot({ limits: [five] })))).toEqual({ type: "idle" });
  });

  it("stays silent without a projection (estimates, no data)", () => {
    const five = limit(38, { source: "estimate" });
    expect(projectionLine(limitsOf(snapshot({ limits: [five] })))).toBeNull();
    expect(projectionLine(limitsOf(snapshot({ limits: [] })))).toBeNull();
  });
});

describe("sessionList", () => {
  it("lists at most five live sessions and counts the rest", () => {
    const live = Array.from({ length: 8 }, (_, i) => session(i + 1));
    const list = sessionList(snapshot({ live_sessions: live }));
    expect(list.mode).toBe("live");
    expect(list.rows.map((r) => r.session_id)).toEqual([1, 2, 3, 4, 5]);
    expect(list.more).toBe(3);
  });

  it("keeps the backend's order (largest share first)", () => {
    const live = [session(1, { share_percent: 2 }), session(2, { share_percent: 9 })];
    expect(sessionList(snapshot({ live_sessions: live })).rows[0]?.session_id).toBe(1);
  });

  it("shows the window's three largest when nothing runs", () => {
    const top = [1, 2, 3, 4].map((n) => session(n, { status: null }));
    const list = sessionList(snapshot({ live_sessions: [], top_sessions: top }));
    expect(list).toMatchObject({ mode: "top", more: 0 });
    expect(list.rows).toHaveLength(3);
  });
});

describe("notices", () => {
  it("reports indexing progress by bytes", () => {
    const s = snapshot({
      index_state: {
        initial_scan_done: false,
        last_indexed_at: null,
        progress: {
          files_done: 10,
          files_total: 100,
          bytes_done: 429,
          bytes_total: 1_000,
          phase: "indexing",
        },
        recent_ready: true,
        drift: null,
      },
    });
    expect(notices(s)).toEqual([{ type: "indexing", pct: 42, recentReady: true }]);
  });

  it("is calm about a stale token and schema drift", () => {
    const s = snapshot({
      api: { ...snapshot().api, state: { state: "token_stale" } },
      index_state: { ...snapshot().index_state, drift: { unknown_count: 3, last_at: NOW } },
    });
    expect(notices(s).map((n) => n.type)).toEqual(["token-stale", "drift"]);
  });

  it("mentions being offline only when the values depend on the API", () => {
    const offline = { ...snapshot().api, state: { state: "offline" as const, retry_at: NOW } };
    expect(notices(snapshot({ api: offline }))).toEqual([]);
    const fromApi = [limit(42, { source: "api" })];
    expect(notices(snapshot({ api: offline, limits: fromApi }))).toEqual([
      { type: "offline", since: fromApi[0]?.observed_at },
    ]);
  });
});

describe("emptyState", () => {
  it("never invents a percentage", () => {
    expect(emptyState(snapshot({ limits: [] }))).toBe("no-limits");
    expect(emptyState(snapshot({ account: null }))).toBe("no-claude");
    expect(emptyState(snapshot())).toBeNull();
  });
});

describe("refreshState", () => {
  it("is hidden without API consent", () => {
    const off = { ...snapshot().api, state: { state: "off" as const } };
    expect(refreshState(snapshot({ api: off }), NOW)).toEqual({ shown: false });
  });

  it("waits a minute after the last request", () => {
    const api = { ...snapshot().api };
    api.diagnostics = { ...api.diagnostics, last_attempt: NOW - 30_000 };
    expect(refreshState(snapshot({ api }), NOW)).toMatchObject({
      enabled: false,
      reason: "cooldown",
    });
    expect(refreshState(snapshot({ api }), NOW + 31_000)).toEqual({ shown: true, enabled: true });
  });

  it("is disabled while the client is blocked", () => {
    const stale = { ...snapshot().api, state: { state: "token_stale" as const } };
    expect(refreshState(snapshot({ api: stale }), NOW)).toMatchObject({ reason: "blocked" });
  });
});
