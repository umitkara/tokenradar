// Synthetic projects for the gallery (Step 16b): shaped like `list_projects`, `project_tree` and
// `get_project`, never real usage. Names, folders and amounts are made up; the tree is written
// out by hand in the backend's order (depth first, largest cost first).

import type {
  ProjectDetail,
  ProjectNode,
  ProjectRow,
  ProjectWeek,
  SessionRow,
} from "../../lib/ipc/bindings";
import { sessionsDemo } from "./sessions";

const HOUR = 3_600_000;
const DAY = 24 * HOUR;
const WEEK = 7 * DAY;
const USD = 1_000_000;

const FOLDERS = [
  { name: "orbit-api", path: "D:\\Work\\Clients\\Orbit\\orbit-api" },
  { name: "orbit-admin", path: "D:\\Work\\Clients\\Orbit\\orbit-admin" },
  { name: "lumen-web", path: "D:\\Work\\Lumen\\lumen-web" },
  { name: "field-notes", path: "C:\\Users\\ada\\Documents\\field-notes" },
  { name: "kestrel", path: "D:\\Work\\kestrel" },
  {
    name: "kestrel-docs-site-with-a-rather-long-folder-name",
    path: "D:\\Work\\kestrel\\docs\\kestrel-docs-site-with-a-rather-long-folder-name",
  },
];

function trend(seed: number, scale: number): number[] {
  return Array.from({ length: 8 }, (_, i) =>
    Math.round(((Math.sin(seed + i * 1.3) + 1.2) / 2.2) * scale * USD),
  );
}

function row(i: number, now: number): ProjectRow {
  const f = FOLDERS[i % FOLDERS.length] ?? { name: "?", path: "?" };
  const round = Math.floor(i / FOLDERS.length);
  const t = trend(i, 40 / (1 + i * 0.4));
  return {
    id: 100 + i,
    name: round === 0 ? f.name : `${f.name}-${round}`,
    path: round === 0 ? f.path : `${f.path}-${round}`,
    sessions: 3 + ((i * 7) % 40),
    last_active: now - (i * 5 + 1) * HOUR,
    cost: t.slice(-4).reduce((a, b) => a + b, 0),
    weekly_percent: i % 5 === 3 ? null : Math.round((18 / (1 + i)) * 10) / 10,
    trend: t,
  };
}

export function projectsDemo(now: number, count = 24): ProjectRow[] {
  return Array.from({ length: count }, (_, i) => row(i, now));
}

function node(
  depth: number,
  label: string,
  path: string,
  children: number,
  rows: ProjectRow[],
  project: ProjectRow | null = null,
): ProjectNode {
  const sum = (f: (r: ProjectRow) => number) => rows.reduce((a, r) => a + f(r), 0);
  const weekly = rows.filter((r) => r.weekly_percent != null);
  return {
    depth,
    label,
    path,
    project,
    projects: rows.length,
    sessions: sum((r) => r.sessions),
    last_active: Math.max(...rows.map((r) => r.last_active ?? 0)),
    cost: sum((r) => r.cost),
    weekly_percent: weekly.length ? sum((r) => r.weekly_percent ?? 0) : null,
    trend: Array.from({ length: 8 }, (_, w) => sum((r) => r.trend[w] ?? 0)),
    children,
  };
}

export function projectTreeDemo(now: number): ProjectNode[] {
  const [api, admin, lumen, notes, kestrel, docs] = FOLDERS.map((_, i) => row(i, now)) as [
    ProjectRow,
    ProjectRow,
    ProjectRow,
    ProjectRow,
    ProjectRow,
    ProjectRow,
  ];
  return [
    node(0, "D:\\Work", "D:\\Work", 3, [api, admin, lumen, kestrel, docs]),
    node(1, "Clients\\Orbit", "D:\\Work\\Clients\\Orbit", 2, [api, admin]),
    node(2, api.name, api.path, 0, [api], api),
    node(2, admin.name, admin.path, 0, [admin], admin),
    node(1, lumen.name, lumen.path, 0, [lumen], lumen),
    node(1, kestrel.name, kestrel.path, 1, [kestrel, docs], kestrel),
    node(2, `docs\\${docs.name}`, docs.path, 0, [docs], docs),
    node(0, "C:\\Users\\ada\\Documents\\field-notes", notes.path, 0, [notes], notes),
  ];
}

export function projectDetailDemo(now: number): ProjectDetail {
  const project = row(0, now);
  const weekStart = now - 2 * DAY;
  const weeks: ProjectWeek[] = Array.from({ length: 6 }, (_, i) => {
    const starts = weekStart - (5 - i) * WEEK;
    return {
      window_id: 500 + i,
      starts_at: starts,
      resets_at: starts + WEEK,
      estimated: i < 2,
      percent: [2.1, 4.8, 9.6, 6.2, 12.4, 3.9][i] ?? null,
      cost: Math.round(([3, 7, 15, 9, 21, 6][i] ?? 0) * USD),
    };
  });
  const sessions = sessionsDemo(now, 5)
    .map((s) => ({ ...s, project: { id: project.id, name: project.name } }))
    .sort((a, b) => b.cost - a.cost);
  return {
    project,
    weeks,
    models: [
      { model: "claude-opus-5-5", family: "opus", cost: 41 * USD, tokens: 9_400_000 },
      { model: "claude-fable-5-2", family: "fable", cost: 12 * USD, tokens: 2_100_000 },
      { model: "claude-sonnet-5-5", family: "sonnet", cost: 4 * USD, tokens: 1_800_000 },
      {
        model: "claude-haiku-4-5-20251001",
        family: "haiku",
        cost: Math.round(0.6 * USD),
        tokens: 900_000,
      },
    ],
    top_sessions: sessions,
  };
}

/** The detail's embedded sessions list: every session of `projectDetailDemo`'s project. */
export function projectSessionsDemo(now: number): SessionRow[] {
  const project = row(0, now);
  return sessionsDemo(now, 12).map((s) => ({
    ...s,
    project: { id: project.id, name: project.name },
  }));
}
