// Synthetic windows and patterns for the gallery (Step 17a): shaped like `list_weeks`,
// `window_strip`, `list_windows`, `get_window`, `get_heatmap`, `limit_risk` and
// `model_mix_trend`, never real usage. Every number comes from a seeded generator.

import type {
  Basis,
  ContribSeries,
  CycleRow,
  HeatCell,
  Heatmap,
  LimitRisk,
  MixTrend,
  StripDay,
  WeekCycles,
  WindowDetail,
  WindowRow,
  WindowStrip,
} from "../../lib/ipc/bindings";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const WEEK = 7 * DAY;
const USD = 1_000_000;

function rng(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s * 1_664_525 + 1_013_904_223) >>> 0;
    return s / 2 ** 32;
  };
}

const BASIS: Basis = {
  method: "calibrated",
  confidence: 0.72,
  approx: true,
  dim: false,
  stale: false,
};
const PROJECTS = ["orbit-api", "lumen-web", "kestrel", "field-notes"];

function weekly(i: number, now: number, r: () => number): WindowRow {
  const resets = now - (now % HOUR) + 2 * DAY + 5 * HOUR - i * WEEK;
  const peak = i === 0 ? 46 : i === 2 ? 100 : Math.round(40 + r() * 55);
  return {
    id: 900 - i,
    kind: "weekly_all",
    scope: null,
    starts_at: resets - WEEK,
    resets_at: resets,
    peak_percent: i === 5 ? Math.round(peak * 0.8) : peak,
    hit_limit: i === 5 ? null : peak >= 100,
    estimated: i === 5,
    open: i === 0,
    cost: Math.round((300 + r() * 500) * USD),
    sessions: 20 + Math.floor(r() * 60),
    other_percent: Math.round(r() * 120) / 10,
    basis: BASIS,
  };
}

export function weeksDemo(now: number): WeekCycles {
  const r = rng(17);
  const items: CycleRow[] = Array.from({ length: 8 }, (_, i) => {
    const w = weekly(i, now, r);
    const days = Array.from({ length: 7 }, (_, d) => {
      const starts_at = w.starts_at + d * DAY;
      const future = starts_at > now;
      const pct = future ? 0 : Math.round(r() * 160) / 10;
      return {
        starts_at,
        percent: future ? null : pct,
        cost: future ? 0 : Math.round(pct * 9 * USD),
        future,
      };
    });
    const hit = w.hit_limit ? w.starts_at + 5 * DAY + 14 * HOUR : null;
    return {
      window: w,
      days,
      hit_at: hit,
      hit_day: hit == null ? null : 5,
      projects: PROJECTS.slice(0, 3).map((name, k) => ({
        project: k === 2 && i % 3 === 1 ? null : { id: 100 + k, name },
        percent: Math.round((18 - k * 5 + r() * 4) * 10) / 10,
        cost: Math.round((120 - k * 30) * USD),
      })),
      breakdown: [
        { key: "claude_code", percent: Math.round(70 + r() * 20) },
        { key: "chat", percent: Math.round(r() * 15) },
      ],
      scoped: [
        {
          window_id: 1900 - i,
          scope: { model: "Fable" },
          peak_percent: Math.round(r() * 70),
          hit_limit: false,
          estimated: false,
        },
      ],
    };
  });
  return { items, scopes: [{ model: "Fable" }], day_basis: BASIS };
}

export function stripDemo(now: number): WindowStrip {
  const r = rng(5);
  const windows: WindowStrip["windows"] = [];
  const days: StripDay[] = [];
  const today = new Date(now);
  today.setHours(0, 0, 0, 0);
  for (let d = 13; d >= 0; d--) {
    const day = today.getTime() - d * DAY;
    const segments: StripDay["segments"] = [];
    for (const start of [9, 15, 21]) {
      if (r() < 0.3) continue;
      const from = day + start * HOUR + Math.floor(r() * 50) * MINUTE;
      const to = from + 5 * HOUR;
      if (from > now) continue;
      const peak = Math.round(20 + r() * 85);
      const index = windows.length;
      windows.push({
        window: {
          id: 5000 + index,
          kind: "session",
          scope: null,
          starts_at: from,
          resets_at: to,
          peak_percent: Math.min(100, peak),
          hit_limit: peak >= 100,
          estimated: index % 9 === 4,
          open: to > now,
          cost: Math.round(peak * 0.6 * USD),
          sessions: 1 + Math.floor(r() * 5),
          other_percent: null,
          basis: BASIS,
        },
        top: [
          {
            name: { text: "Refactor the indexer", fallback: false },
            percent: Math.round(peak * 0.5),
          },
          {
            name: { text: "lumen-web · 3f2a91c0", fallback: true },
            percent: Math.round(peak * 0.2),
          },
        ],
      });
      const fromMin = Math.round((from - day) / MINUTE);
      segments.push({
        window: index,
        from_minute: fromMin,
        to_minute: Math.min(1440, fromMin + 300),
      });
    }
    days.push({ day, minutes: 1440, segments });
  }
  // Windows that run past midnight continue on the next day.
  for (const [i, d] of days.entries()) {
    const next = days[i + 1];
    for (const s of d.segments) {
      const w = windows[s.window];
      if (!w || !next || w.window.resets_at <= next.day) continue;
      next.segments.unshift({
        window: s.window,
        from_minute: 0,
        to_minute: Math.round((w.window.resets_at - next.day) / MINUTE),
      });
    }
  }
  return { any: true, windows, days: days.reverse() };
}

export function windowRowsDemo(now: number): WindowRow[] {
  const r = rng(29);
  const rows = Array.from({ length: 8 }, (_, i) => weekly(i, now, r));
  const strip = stripDemo(now).windows.map((w) => w.window);
  return [...rows, ...strip].sort((a, b) => b.starts_at - a.starts_at);
}

export function windowDetailDemo(now: number): WindowDetail {
  const w = weekly(2, now, rng(17));
  const at = Array.from({ length: 85 }, (_, i) => w.starts_at + i * 2 * HOUR);
  const names = [
    "Refactor the indexer",
    "Write the export tests",
    "Release notes",
    "orbit-api · 1c9e02ab",
  ];
  const shares = [31, 22, 14, 9];
  const series: ContribSeries[] = names.map((text, k) => ({
    session_id: 2000 + k,
    name: { text, fallback: k === 3 },
    values: at.map(
      (_, i) => Math.round((shares[k] ?? 0) * Math.min(1, Math.max(0, (i - k * 8) / 60)) * 10) / 10,
    ),
  }));
  series.push({
    session_id: null,
    name: null,
    values: at.map((_, i) => Math.round(12 * (i / 84) * 10) / 10),
  });
  return {
    window: w,
    shares: names.map((text, k) => ({
      session_id: 2000 + k,
      name: { text, fallback: k === 3 },
      project: { id: 100 + k, name: PROJECTS[k] ?? "" },
      percent: shares[k] ?? 0,
      cost: Math.round((shares[k] ?? 0) * 8 * USD),
    })),
    other: {
      percent: 12,
      share_of_total: 12,
      breakdown_other: 14,
      suspect: false,
      other_excess: false,
    },
    points: at.map((t, i) => ({
      at: t,
      percent: Math.min(100, Math.round(i * 1.25)),
      source: "statusline",
    })),
    contrib: { unit: "percent", at, series },
    alerts: [
      { rule: "threshold", threshold: 75, at: w.starts_at + 4 * DAY },
      { rule: "projection", threshold: null, at: w.starts_at + 4 * DAY + 6 * HOUR },
      { rule: "threshold", threshold: 90, at: w.starts_at + 5 * DAY },
    ],
  };
}

export function heatmapDemo(now: number): Heatmap {
  const r = rng(11);
  const cells: HeatCell[] = [];
  for (let weekday = 0; weekday < 7; weekday++) {
    for (let hour = 0; hour < 24; hour++) {
      const work = weekday < 5 && hour >= 9 && hour <= 18;
      const evening = hour >= 20 && hour <= 23;
      const base = work ? 8 : evening ? 4 : weekday >= 5 && hour >= 11 && hour <= 17 ? 3 : 0.2;
      const total = r() < 0.15 ? 0 : Math.round(base * (0.5 + r()) * USD);
      const days = total > 0 ? 1 + Math.floor(r() * 11) : 0;
      cells.push({
        weekday,
        hour,
        total,
        mean: days ? Math.round(total / days) : 0,
        active_days: days,
      });
    }
  }
  const hours = Array.from({ length: 24 }, (_, hour) => {
    const wd = cells
      .filter((c) => c.hour === hour && c.weekday < 5)
      .reduce((a, c) => a + (c.total ?? 0), 0);
    const we = cells
      .filter((c) => c.hour === hour && c.weekday >= 5)
      .reduce((a, c) => a + (c.total ?? 0), 0);
    return { hour, weekday: Math.round(wd / 60), weekend: Math.round(we / 24) };
  });
  return {
    metric: "cost",
    range: { from: now - 84 * DAY, to: now },
    cells,
    max_total: Math.max(...cells.map((c) => c.total ?? 0)),
    basis: null,
    hours,
  };
}

export function riskDemo(): LimitRisk {
  const r = rng(3);
  const hours = Array.from({ length: 24 }, (_, hour) => {
    const windows = hour >= 8 && hour <= 22 ? 2 + Math.floor(r() * 6) : Math.floor(r() * 2);
    const hit =
      hour >= 13 && hour <= 15 ? Math.ceil(windows * 0.6) : Math.floor(windows * r() * 0.3);
    return { hour, windows, hit, high: Math.min(windows, hit + Math.floor(r() * 2)) };
  });
  const windows = hours.reduce((a, h) => a + h.windows, 0);
  const hit = hours.reduce((a, h) => a + h.hit, 0);
  const span = hours.slice(13, 16);
  return {
    any: true,
    windows,
    hit,
    enough: true,
    minimum: 10,
    hours,
    span: {
      from_hour: 13,
      to_hour: 16,
      windows: span.reduce((a, h) => a + h.windows, 0),
      hit: span.reduce((a, h) => a + h.hit, 0),
    },
  };
}

export function mixDemo(now: number): MixTrend {
  const r = rng(23);
  const start = now - 11 * WEEK;
  return {
    families: ["opus", "fable", "sonnet", "haiku", null],
    weeks: Array.from({ length: 12 }, (_, i) => {
      const opus = (60 - i * 2 + r() * 10) * USD;
      const fable = (i * 3 + r() * 8) * USD;
      const sonnet = (20 + r() * 10) * USD;
      const haiku = (4 + r() * 3) * USD;
      const other = r() * 2 * USD;
      const parts = [opus, fable, sonnet, haiku, other].map(Math.round);
      return { starts_at: start + i * WEEK, total: parts.reduce((a, b) => a + b, 0), parts };
    }),
  };
}
