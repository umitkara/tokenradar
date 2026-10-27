// What the panel shows, picked from the backend's `PanelSnapshot`. Selection only: every
// percentage, severity, projection and share comes from the core; this decides which of them
// fills which slot. Plain functions so they are unit tested without a DOM.

import type {
  AccountView,
  ApiState,
  LimitView,
  PanelSnapshot,
  SessionShareView,
} from "../../lib/ipc/bindings";
import { indexPercent } from "../../lib/progress";

/** Live sessions listed before "+N more". */
export const MAX_LIVE = 5;
/** Largest finished sessions listed when nothing runs. */
export const MAX_TOP = 3;
/** The usage API's minimum interval between requests (architecture.md §10). */
export const REFRESH_COOLDOWN_MS = 60_000;

export interface Limits {
  fiveHour: LimitView | null;
  weekly: LimitView | null;
  /** Weekly limits of one model or surface, under the weekly row. */
  scoped: LimitView[];
}

export function limitsOf(snapshot: PanelSnapshot): Limits {
  const find = (kind: string) => snapshot.limits.find((l) => l.kind === kind) ?? null;
  return {
    fiveHour: find("session"),
    weekly: find("weekly_all"),
    scoped: snapshot.limits.filter((l) => l.kind === "weekly_scoped"),
  };
}

/** "Max 20x" from `default_claude_max_20x`, else the plan name capitalized. Product names
 *  are not translated. */
export function planLabel(account: AccountView | null): string | null {
  if (!account) return null;
  const multiplier = account.tier?.match(/max_(\d+)x/)?.[1];
  if (multiplier) return `Max ${multiplier}x`;
  if (!account.plan) return null;
  return account.plan.charAt(0).toUpperCase() + account.plan.slice(1);
}

export type ProjectionLine =
  | {
      type: "will-hit";
      kind: "five_hour" | "weekly";
      etaAt: number;
      resetAt: number;
      low: boolean;
    }
  | { type: "at-reset"; kind: "five_hour" | "weekly"; pct: number; low: boolean }
  | { type: "idle" };

/**
 * The one projection the panel states. A limit that fills before its reset wins (5-hour
 * first); otherwise the 5-hour window's level at the reset, or "no usage" once it stopped.
 */
export function projectionLine(limits: Limits): ProjectionLine | null {
  const candidates = [
    ["five_hour", limits.fiveHour],
    ["weekly", limits.weekly],
  ] as const;
  for (const [kind, limit] of candidates) {
    const p = limit?.projection;
    if (p?.will_hit_before_reset && p.eta_100_at != null && limit?.resets_at != null) {
      return {
        type: "will-hit",
        kind,
        etaAt: p.eta_100_at,
        resetAt: limit.resets_at,
        low: p.confidence === "low",
      };
    }
  }
  const p = limits.fiveHour?.projection;
  if (!p) return null;
  if (p.eta_100_at == null) return { type: "idle" };
  if (p.projected_at_reset == null) return null;
  return {
    type: "at-reset",
    kind: "five_hour",
    pct: p.projected_at_reset,
    low: p.confidence === "low",
  };
}

export interface SessionList {
  /** `live`: running sessions; `top`: nothing runs, the window's largest instead. */
  mode: "live" | "top";
  rows: SessionShareView[];
  /** Rows not shown. */
  more: number;
}

/** The backend sorts both lists, largest share first. */
export function sessionList(snapshot: PanelSnapshot): SessionList {
  if (snapshot.live_sessions.length > 0) {
    const rows = snapshot.live_sessions.slice(0, MAX_LIVE);
    return { mode: "live", rows, more: snapshot.live_sessions.length - rows.length };
  }
  return { mode: "top", rows: snapshot.top_sessions.slice(0, MAX_TOP), more: 0 };
}

export type Notice =
  | { type: "indexing"; pct: number; recentReady: boolean }
  | { type: "token-stale" }
  | { type: "offline"; since: number | null }
  | { type: "drift" };

/** Calm notices above the limits (design-language.md §9), most useful first. */
export function notices(snapshot: PanelSnapshot): Notice[] {
  const out: Notice[] = [];
  const index = snapshot.index_state;
  if (!index.initial_scan_done && index.progress) {
    out.push({
      type: "indexing",
      pct: indexPercent(index.progress),
      recentReady: index.recent_ready,
    });
  }
  const api = snapshot.api.state.state;
  if (api === "token_stale") out.push({ type: "token-stale" });
  // Live statusline readings do not need the network: offline only matters for API values.
  const primary = primaryLimit(snapshot);
  if (api === "offline" && primary?.source !== "statusline") {
    out.push({
      type: "offline",
      since: primary?.observed_at ?? snapshot.api.diagnostics.last_success,
    });
  }
  if (index.drift) out.push({ type: "drift" });
  return out;
}

/** What replaces the limits section when there is nothing honest to draw. */
export type Empty = "no-claude" | "no-limits" | null;

export function emptyState(snapshot: PanelSnapshot): Empty {
  if (!snapshot.account) return "no-claude";
  if (snapshot.limits.length === 0) return "no-limits";
  return null;
}

/** The value the footer's source badge describes: the 5-hour window, else any limit. */
export function primaryLimit(snapshot: PanelSnapshot): LimitView | null {
  const limits = limitsOf(snapshot);
  return limits.fiveHour ?? limits.weekly ?? snapshot.limits[0] ?? null;
}

export type Refresh =
  | { shown: false }
  | { shown: true; enabled: true }
  | { shown: true; enabled: false; reason: "cooldown" | "blocked" };

/**
 * "Refresh" asks the usage API, so it exists only with API consent. It waits out the minimum
 * interval since the last request and does nothing while the client is blocked.
 */
export function refreshState(
  snapshot: PanelSnapshot,
  now: number,
  /** A click the snapshot does not reflect yet. */
  clickedAt: number | null = null,
): Refresh {
  const state: ApiState["state"] = snapshot.api.state.state;
  if (state === "off") return { shown: false };
  if (state === "token_stale" || state === "backoff") {
    return { shown: true, enabled: false, reason: "blocked" };
  }
  const last = Math.max(snapshot.api.diagnostics.last_attempt ?? 0, clickedAt ?? 0);
  if (last > 0 && now - last < REFRESH_COOLDOWN_MS) {
    return { shown: true, enabled: false, reason: "cooldown" };
  }
  return { shown: true, enabled: true };
}

/** Running sessions for the header. */
export function activeCount(snapshot: PanelSnapshot): number {
  return snapshot.live_sessions.length;
}
