// Synthetic sessions for the gallery (Step 16a): shaped like `list_sessions` and `get_session`,
// never real usage. Screenshots and the 10,000-row frame-time measurement come from here, so
// names, folders and amounts are made up.

import type { ImpactRow, SessionDetail, SessionRow, TimelineBucket } from "../../lib/ipc/bindings";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const USD = 1_000_000;

const TITLES = [
  "Refactor the indexer's chunk coordinator",
  "Add keyset paging to the project list",
  "Fix flaky watcher test on network drives",
  "Write the calibration notes for the weekly window and explain why the proportional split is approximate",
  "Tray icon contrast in high-contrast mode",
  "Bump uPlot and check the bundle size",
  "Investigate slow cold start on Windows 10",
  "Sketch the statusline bridge install flow",
];
const PROJECTS = [
  { id: 1, name: "orbit-api" },
  { id: 2, name: "lumen-web" },
  { id: 3, name: "field-notes" },
  { id: 4, name: "kestrel" },
];
const MODELS = [
  ["claude-opus-5-5"],
  ["claude-opus-5-5", "claude-haiku-4-5-20251001"],
  ["claude-sonnet-5-5"],
  ["claude-fable-5-2", "claude-opus-5-5", "claude-haiku-4-5-20251001"],
];

/** A deterministic pseudo-random sequence, so every render shows the same rows. */
function rng(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s * 1_664_525 + 1_013_904_223) >>> 0;
    return s / 2 ** 32;
  };
}

export function sessionsDemo(now: number, count = 40): SessionRow[] {
  const r = rng(16);
  const rows: SessionRow[] = [];
  let last = now - 4 * MINUTE;
  for (let i = 0; i < count; i++) {
    const length = (10 + r() * 240) * MINUTE;
    const first = last - length;
    const cost = Math.round((0.2 + r() * r() * 30) * USD);
    const fallback = i % 13 === 7;
    const project = PROJECTS[i % PROJECTS.length] ?? null;
    rows.push({
      id: 1000 + i,
      uuid: `00000000-0000-4000-8000-${String(i).padStart(12, "0")}`,
      name: fallback
        ? { text: `${project?.name ?? "?"} · 0000${String(i).padStart(4, "0")}`, fallback: true }
        : { text: TITLES[i % TITLES.length] ?? "", fallback: false },
      project,
      first_ts: first,
      last_ts: last,
      live: i < 2,
      models: MODELS[i % MODELS.length] ?? [],
      cost,
      tokens: {
        input: Math.round(cost / 40),
        output: Math.round(cost / 12),
        cache_read: Math.round(cost * 1.8),
        cache_write: Math.round(cost / 6),
      },
      subagents: i % 3 === 0 ? Math.round(r() * 6) : 0,
      weekly_percent: i % 9 === 4 ? null : Math.round((cost / USD) * 0.6 * 10) / 10,
      session_percent: null,
      impact_basis: {
        method: "proportional",
        confidence: i % 4 === 0 ? 0.42 : 0.81,
        approx: true,
        dim: i % 4 === 0,
        stale: false,
      },
      transcript_deleted: i % 11 === 10,
    });
    last = first - (5 + r() * 600) * MINUTE;
  }
  return rows;
}

export function sessionDetailDemo(now: number): SessionDetail {
  const start = now - 3.2 * HOUR;
  const bucket = 5 * MINUTE;
  const r = rng(5);
  const timeline: TimelineBucket[] = [];
  for (let at = start - (start % bucket); at < now; at += bucket) {
    // A lunch-sized pause in the middle.
    if (at > start + 1.3 * HOUR && at < start + 1.8 * HOUR) continue;
    const busy = 0.3 + r();
    timeline.push({
      at,
      model: "claude-opus-5-5",
      family: "opus",
      cost: Math.round(busy * 0.42 * USD),
    });
    if (r() > 0.55) {
      timeline.push({
        at,
        model: "claude-haiku-4-5-20251001",
        family: "haiku",
        cost: Math.round(r() * 0.05 * USD),
      });
    }
    if (at > start + 2.3 * HOUR && r() > 0.4) {
      timeline.push({
        at,
        model: "claude-fable-5-2",
        family: "fable",
        cost: Math.round(r() * 0.3 * USD),
      });
    }
  }
  const cost = timeline.reduce((a, b) => a + b.cost, 0);
  const basis = {
    method: "proportional" as const,
    confidence: 0.81,
    approx: true,
    dim: false,
    stale: false,
  };
  const weeklyStart = now - 4.5 * 24 * HOUR;
  const impact: ImpactRow[] = [
    {
      window_id: 9,
      kind: "session",
      scope: null,
      starts_at: start - 0.5 * HOUR,
      resets_at: start + 4.5 * HOUR,
      peak_percent: 100,
      hit_limit: true,
      estimated: false,
      percent: 71,
      cost: Math.round(cost * 0.8),
      basis,
    },
    {
      window_id: 7,
      kind: "weekly_all",
      scope: null,
      starts_at: weeklyStart,
      resets_at: weeklyStart + 7 * 24 * HOUR,
      peak_percent: 58,
      hit_limit: false,
      estimated: false,
      percent: 6.4,
      cost: Math.round(cost * 0.92),
      basis,
    },
    {
      window_id: 8,
      kind: "weekly_scoped",
      scope: { model: "Fable" },
      starts_at: weeklyStart,
      resets_at: weeklyStart + 7 * 24 * HOUR,
      peak_percent: 21,
      hit_limit: false,
      estimated: false,
      percent: 3.1,
      cost: Math.round(cost * 0.18),
      basis: { ...basis, confidence: 0.38, dim: true },
    },
  ];
  const row = sessionsDemo(now, 1)[0] as SessionRow;
  return {
    session: {
      ...row,
      name: { text: TITLES[3] ?? "", fallback: false },
      first_ts: start,
      last_ts: now - 2 * MINUTE,
      models: ["claude-opus-5-5", "claude-fable-5-2", "claude-haiku-4-5-20251001"],
      cost,
      subagents: 3,
      weekly_percent: 6.4,
      transcript_deleted: false,
    },
    git_branch: "feat/calibration-notes",
    cc_version: "2.4.1",
    entrypoint: "cli",
    path: "D:/work/orbit-api",
    cwds: [
      { path: "D:\\work\\orbit-api", first_ts: start, last_ts: now - 30 * MINUTE },
      {
        path: "D:\\work\\orbit-api\\crates\\core",
        first_ts: start + HOUR,
        last_ts: now - 2 * MINUTE,
      },
    ],
    efforts: ["high", "medium"],
    fast: true,
    timeline,
    bucket_ms: bucket,
    subagents: [
      {
        id: 2001,
        uuid: "00000000-0000-4000-8000-000000002001",
        title: "Explore the calibration module",
        cost: Math.round(0.84 * USD),
        tokens: { input: 21_000, output: 4_800, cache_read: 310_000, cache_write: 18_000 },
        models: ["claude-haiku-4-5-20251001"],
      },
      {
        id: 2002,
        uuid: "00000000-0000-4000-8000-000000002002",
        title: "Review the proportional split",
        cost: Math.round(2.1 * USD),
        tokens: { input: 48_000, output: 9_900, cache_read: 720_000, cache_write: 41_000 },
        models: ["claude-opus-5-5"],
      },
      {
        id: 2003,
        uuid: "00000000-0000-4000-8000-000000002003",
        title: null,
        cost: Math.round(0.31 * USD),
        tokens: { input: 6_100, output: 1_200, cache_read: 88_000, cache_write: 5_000 },
        models: ["claude-haiku-4-5-20251001"],
      },
    ],
    cache: {
      hit_ratio: 0.912,
      write_5m: 1_840_000,
      write_1h: 260_000,
      savings: Math.round(cost * 0.61),
    },
    impact,
  };
}
