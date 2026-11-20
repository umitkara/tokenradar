// A synthetic overview for the gallery (Step 15b): shaped like `get_overview`, never real usage.
// Screenshots in the docs come from here, so names and amounts are made up.

import type {
  Overview,
  SeriesPoint,
  SessionMarker,
  TopProject,
  TopSession,
  WindowBand,
} from "../../lib/ipc/bindings";
import { weekly } from "./fixtures";

const HOUR = 3_600_000;
const DAY = 24 * HOUR;
const USD = 1_000_000;

/** A weekly window that reset 4.5 days ago, used in the daytime only. */
function weeklySeries(now: number): { points: SeriesPoint[]; windows: WindowBand[] } {
  const reset = now - 4.5 * DAY;
  const points: SeriesPoint[] = [];
  let value = 31;
  for (let at = now - 7 * DAY; at <= now; at += HOUR) {
    if (at >= reset && at < reset + HOUR) value = 0;
    const hour = new Date(at).getHours();
    if (hour >= 9 && hour < 22) value += at < reset ? 0.5 : 0.75;
    points.push({ at, percent: Math.round(value * 10) / 10, source: "statusline" });
  }
  return {
    points,
    windows: [
      {
        window_id: 41,
        starts_at: reset - 7 * DAY,
        resets_at: reset,
        peak_percent: 74,
        hit_limit: false,
        estimated: false,
      },
      {
        window_id: 42,
        starts_at: reset,
        resets_at: reset + 7 * DAY,
        peak_percent: null,
        hit_limit: null,
        estimated: false,
      },
    ],
  };
}

const names = [
  ["Index rebuild after schema change", "acme-api", 8.4, 96.2],
  ["Tray icon flicker fix", "desktop-shell", 5.1, 61.4],
  ["Migrate billing tables", "acme-api", 3.2, 38.9],
  ["Docs pass for the CLI", "toolbox", 1.6, 17.3],
  ["Try a smaller model for tests", "toolbox", 0.9, 9.8],
] as const;

function daily(seed: number): number[] {
  return [0, 1, 2, 3, 4, 5, 6].map((d) => Math.round((((seed * (d + 3)) % 7) + d) * 0.6 * USD));
}

export function overviewDemo(): Overview {
  const now = Date.now();
  const { points, windows } = weeklySeries(now);
  const markers: SessionMarker[] = names.map(([text, , percent], i) => ({
    session_id: i + 1,
    name: { text, fallback: false },
    start: now - (6 - i * 1.3) * DAY,
    end: now - (6 - i * 1.3) * DAY + (i === 3 ? 0.1 : 0.3 + i * 0.1) * DAY,
    percent,
  }));
  const topSessions: TopSession[] = names.map(([text, project, percent, cost], i) => ({
    session_id: i + 1,
    name: { text, fallback: false },
    project: { id: project.length, name: project },
    percent,
    cost: cost * USD,
    daily: daily(i + 2),
  }));
  const topProjects: TopProject[] = [
    ["acme-api", 14.6, 171.5],
    ["desktop-shell", 6.2, 74.0],
    ["toolbox", 2.9, 31.6],
  ].map(([name, percent, cost], i) => ({
    project: { id: i + 1, name: String(name) },
    percent: Number(percent),
    cost: Number(cost) * USD,
    daily: daily(i + 4),
  }));
  const current = weekly(28, { resetsIn: 2.5 * DAY, projected: 61 });
  return {
    limits: [
      {
        ...current,
        calibration: { confidence: 0.72, level: "medium", n_obs: 38, drift: null },
      },
    ],
    absent: [{ kind: "session", scope: null }],
    week: { starts_at: now - 4.5 * DAY, ends_at: now + 2.5 * DAY, window_id: 42 },
    series: [
      {
        limit: { kind: "weekly_all", scope: null },
        range: { from: now - 7 * DAY, to: now },
        points,
        windows,
        markers,
      },
    ],
    cost: 284.6 * USD,
    previous_cost: 412.3 * USD,
    previous_cost_to_date: 251.2 * USD,
    change: 284.6 / 251.2 - 1,
    tokens: {
      tokens: { input: 412_000, output: 1_940_000, cache_read: 96_400_000, cache_write: 6_100_000 },
      cache_write_5m: 4_000_000,
      cache_write_1h: 2_100_000,
    },
    models: [
      { model: "claude-opus-5-5", family: "opus", cost: 231.4 * USD, tokens: 81_000_000 },
      { model: "claude-sonnet-5-5", family: "sonnet", cost: 49.8 * USD, tokens: 22_000_000 },
      { model: "claude-haiku-5-5", family: "haiku", cost: 3.4 * USD, tokens: 1_850_000 },
    ],
    top_sessions: topSessions,
    top_projects: topProjects,
    basis: { method: "calibrated", confidence: 0.72, approx: true, dim: false, stale: false },
    other: {
      other: {
        percent: 3.1,
        share_of_total: 11.1,
        breakdown_other: 9.0,
        suspect: false,
        other_excess: false,
      },
      breakdown: [
        ["claude_code", 91],
        ["chat", 7],
        ["cowork", 2],
      ],
    },
    calibration: { confidence: 0.72, level: "medium", n_obs: 38, drift: null },
    plan: {
      month_cost: 812.4 * USD,
      month_start: now - 19 * DAY,
      plan: "Max 20x",
      fee: 200 * USD,
      fee_source: "table",
      ratio: 812.4 / 200,
      month_elapsed: 0.63,
      previous_month_cost: 1_164 * USD,
      previous_ratio: 1_164 / 200,
    },
    generated_at: now,
  };
}
