// What the projects pages draw (Step 16b), as pure functions. The backend sends the rows, the
// tree with its totals and each project's weeks; these only decide what is visible and how
// long a bar is.

import type { ProjectNode, ProjectWeek } from "../../../lib/ipc/bindings";

/**
 * The tree rows a person can see: everything except what lies under a collapsed folder. The
 * nodes come depth first, so a folder's subtree is the run of deeper nodes after it.
 */
export function visibleNodes(
  nodes: readonly ProjectNode[],
  collapsed: ReadonlySet<string>,
): ProjectNode[] {
  const out: ProjectNode[] = [];
  let skipBelow: number | null = null;
  for (const n of nodes) {
    if (skipBelow != null) {
      if (n.depth > skipBelow) continue;
      skipBelow = null;
    }
    out.push(n);
    if (n.children > 0 && collapsed.has(n.path)) skipBelow = n.depth;
  }
  return out;
}

/** The parent of `nodes[index]`: the nearest node before it that is one level up. */
export function parentIndex(nodes: readonly ProjectNode[], index: number): number | null {
  const depth = nodes[index]?.depth ?? 0;
  for (let i = index - 1; i >= 0; i--) {
    if ((nodes[i]?.depth ?? 0) < depth) return i;
  }
  return null;
}

export interface WeekBar {
  key: number;
  starts: number;
  resets: number;
  percent: number | null;
  cost: number;
  estimated: boolean;
  /** Bar height, 0–100 % of the tallest week. */
  height: number;
}

/**
 * The detail's weekly trend: one bar per weekly window, scaled to the largest share. Weeks
 * without a share (no attribution yet) are drawn by cost instead, so an empty trend still
 * shows when the project was used; `byCost` says which.
 */
export function weekBars(weeks: readonly ProjectWeek[]): { bars: WeekBar[]; byCost: boolean } {
  const byCost = weeks.every((w) => w.percent == null);
  const value = (w: ProjectWeek) => (byCost ? w.cost : (w.percent ?? 0));
  const max = Math.max(0, ...weeks.map(value));
  return {
    byCost,
    bars: weeks.map((w) => ({
      key: w.window_id,
      starts: w.starts_at,
      resets: w.resets_at,
      percent: w.percent,
      cost: w.cost,
      estimated: w.estimated,
      height: max > 0 ? (value(w) / max) * 100 : 0,
    })),
  };
}
